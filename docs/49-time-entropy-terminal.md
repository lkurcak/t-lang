# Step 49: time, entropy and the terminal

Code: `core/time.t` (`Duration`), `std/time.t` (`Clock`, `Instant`),
`std/entropy.t` (`Entropy`), `std/io.t` (`Stdin::raw`, `Raw`,
`Stdout::size`), the runtime's `t_clock_*`, `t_entropy_fill` and `t_raw_*`
in `src/cgen/hosted.h`, and the interpreter's in `Terminal`
(`src/interp.rs`) and `Process` (`src/lib.rs`).

The last of the three steps that fill in the `Os`, after bytes, arguments
and the environment (docs/47-io.md), and files (docs/48-files.md).
t-nonograms wanted two things it couldn't have: a seed for its random
puzzles that isn't the same every run, and arrow keys, read as they're
pressed. A hash map wants a key no one can guess, so no one can choose
inputs that collide. This step gives a program the time, entropy, and a
terminal it can use as a screen and a keyboard.

## Each is a capability

The `Os` that `main` is given has two more fields, `clock: Clock` and
`entropy: Entropy`, and its `stdin` has a raw mode.

**Lemma T1 (determinism).** `Clock` and `Entropy` are intrinsic types
with no fields: no code makes one (`std::time::Clock {}` is an error), and
the only ones are in the `Os`. An `Instant` has a field only `std` sees,
so it's made only by a `Clock`. So, as lemma C1 of docs/19-std-io.md said
of streams, a function handed none of the `Os` (and not `with global` or
`unsafe`) does the same whenever it's called with the same arguments: it
can't read the time, or a random number, any more than it can read a file.

```rust,error
fn main() {
    let c = std::time::Clock {}; // error: `Clock` is intrinsic: only its own functions make one or take one apart
    let _ = c.now();
}
```

**Lemma T2 (`Copy` is sound).** Both only tell: reading a clock, or
taking entropy, changes nothing another reader sees. Streams aren't
`Copy`, as the order of two writes is what's written (C2 of
docs/19-std-io.md); two reads of a clock in either order are each a time.
So `Clock` and `Entropy` are `Copy`, `Send` and `Sync`: a program hands
them out freely, to threads too.

## Time

A span of time is a `core::time::Duration`, a whole number of
nanoseconds, never negative, up to `u64`'s greatest, which is more than
584 years. It's in `core`, as it's only a number: code with no clock, and
freestanding code, can use one.

It's made from a unit with a limit: `from_secs` takes at most `MAX_SECS`,
which the checker proves, as with any refinement (docs/08-refinement-types.md),
so making one never panics and never silently wraps. Adding and taking
away are `checked_` or `saturating_`.

```rust
use core::time::{Duration, MAX_SECS};
use std::io::Error;

fn seconds(n: u64) -> Option<Duration> {
    if n <= MAX_SECS { Some(Duration::from_secs(n)) } else { None }
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let d = Duration::from_millis(1_500);
    out.line(&d.as_secs())?;                                        // expect: 1
    out.line(&d.subsec_nanos())?;                                   // expect: 500000000
    out.line(&(d < Duration::from_secs(2)))?;                       // expect: true
    out.line(&d.saturating_sub(Duration::from_secs(2)).is_zero())?; // expect: true
    out.line(&seconds(MAX_SECS + 1).is_none())                      // expect: true
}
```

```rust,error
fn main() {
    let _ = core::time::Duration::from_secs(18_446_744_074); // error: can't prove `n <= MAX_SECS`
}
```

`clock.now()` is an `Instant`, by a clock that only goes forward,
whatever the time of day is set to: how far apart two are is how long
passed between them. `later.since(earlier)` is that `Duration`, or no time
if `later` isn't later, so it never panics; `checked_since` says which.

**Lemma T5 (instants only grow).** `now()` reads the operating system's
monotonic clock (`CLOCK_MONOTONIC`), which never goes back, from any
thread: an `Instant` told after another isn't less.

### Waiting until, never waiting for

`clock.sleep_until(deadline)` returns once `now()` is no earlier than
`deadline`, and at once if it's past. There's no `sleep(duration)`, on
purpose. Most sleeps in programs are bugs: waiting "long enough" for
something else to happen, which should be waited for itself. What's left
is waiting for time itself: polling something that can't tell you when
it changes, backing off before trying again, pacing a game. Each of those
is a deadline, and a loop that waits a second after doing its work drifts
by however long the work took; one that keeps its next deadline, and adds
a second to it, doesn't:

```rust
use core::time::Duration;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let clock = os.clock;
    let start = clock.now();
    let mut next = start;
    let mut tick = 0u32;
    while tick < 3 {
        // Something that takes a while, then the next tick.
        tick = tick + 1;
        next = next.add(Duration::from_millis(10));
        clock.sleep_until(next);
    }
    out.line(&tick)?;                                                  // expect: 3
    out.line(&(clock.now().since(start) >= Duration::from_millis(30))) // expect: true
}
```

`Instant::add` saturates: a deadline past the last moment there is is
one that's never reached, which is what it means.

### The time of day

`clock.unix_time()` is how long it's been since 1970 began, in UTC, by
the operating system's time of day, as a `Duration`: `None` only if the
clock says 1970 hasn't come yet. The time of day can be set, back too, so
it isn't an `Instant`, and the two can't be mixed: how long something
took is measured with `now()`.

## Entropy

`entropy.fill(buf)` fills a slice with bytes no one can guess, as good as
the operating system has for keys (`getentropy`); `entropy.u64()` is
eight of them. Each gives back a `Result`, though on the operating systems
T runs on today it doesn't fail: where there are none to be had, it's
`Unsupported`, and a program decides what to do, rather than the runtime
stopping it, or worse, giving it bytes that can be guessed.

```rust
use std::io::Error;

/// SplitMix64: numbers that look random, from a seed.
#[derive(Discard)]
struct Rng { state: u64 }

impl Rng {
    fn next(&mut self) -> u64 {
        self.state = core::num::wrapping_add(self.state, 0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = core::num::wrapping_mul(z ^ (z >> 30), 0xBF58476D1CE4E5B9);
        z = core::num::wrapping_mul(z ^ (z >> 27), 0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let mut rng = Rng { state: os.entropy.u64()? };
    let die = rng.next() % 6 + 1;
    out.line(&(die >= 1 && die <= 6)) // expect: true
}
```

The generator itself is a pure function of its seed, so it needs no
capability: a test can give it a fixed one, and see the same numbers.

## The terminal

### Raw mode

A terminal starts in line mode: what's typed is shown, and the program
gets a line once Enter is pressed. A game, or an editor, wants each key
as it's pressed, unseen. `stdin.raw()` gives a `Raw`, and while it lives,
the terminal is in raw mode:

- each key is read as it's pressed, and isn't shown;
- Ctrl-C is a key, the byte 3, and doesn't stop the program, which
  decides what it means (decision K1);
- what's written is as before: `\n` still starts a new line, so
  `out.line` works as it did.

The `Raw` is read from, as a `core::io::Read`: a letter is its UTF-8, a
key such as an arrow a few bytes, starting with escape, 27. Run as a test,
the input isn't a terminal, and `raw()` is `Unsupported`:

```rust
use core::io::Read;
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let mut input = os.stdin;
    let mut keys = input.raw()?;
    let mut key = [0u8; 1];
    let _ = keys.read(key.as_mut_slice())?;
    keys.discard();
    out.line(&key[0])
}
// stderr: error: unsupported (os error 25)
```

**Lemma T3 (one raw mode at a time).** A `Raw` holds the program's one
`Stdin` by `&mut`, so while one lives, the borrow checker allows no
other, and no read of the `Stdin` but through it. Its field is private, so
only `raw()` makes one, after putting the terminal in raw mode. So the
runtime needs one place to keep how the terminal was, and what a `Raw`
puts back is what was there before it.

```rust,error
use std::io::Error;

fn main(os: std::Os) -> Result<(), Error> {
    let mut input = os.stdin;
    let a = input.raw()?;
    let b = input.raw()?; // error: borrowed
    a.discard();
    b.discard();
    Ok(())
}
```

**Lemma T4 (the terminal is put back).** However a program in raw mode
stops, but for `SIGKILL`, the terminal is put back as it was:

- dropping the `Raw` puts it back, as its `discard` (docs/20-destructors.md),
  which is total: it can fail only if the terminal's gone, and then
  there's nothing to put back. There's no `forget`, so every `Raw` is
  dropped, and before `main` returns, as it holds what `main` was given;
- a panic stops the program without dropping anything (docs/07-effects.md),
  so the runtime puts the terminal back as it halts;
- `SIGTERM`, `SIGHUP`, `SIGINT` and `SIGQUIT`, each stopping the program
  when it doesn't say otherwise, are caught while it's raw: the terminal
  is put back, then the signal stops it as it would have;
- `SIGKILL` can't be caught: then `reset`, in the shell, puts it back.

The tests run programs on a pseudo-terminal, compiled and interpreted,
and check the terminal after each of these (`tests/terminal.rs`). The
interpreter checks the first as it runs: a program that ends in raw mode
fails, as one that leaks a file does.

### Waiting for a key, or a deadline

`raw.wait(deadline)` waits until a key's been pressed, or `deadline` is
past: `true` if there's one to read, which `read` then gives at once. A
game that animates waits for its next key or its next frame, whichever
comes first, with no thread and no sleep:

```rust,fragment
let mut next = clock.now();
loop {
    next = next.add(Duration::from_millis(100));
    while raw.wait(next)? {
        let _ = raw.read(key.as_mut_slice())?;
        // ... the key moves something ...
    }
    // ... the next frame is drawn ...
}
```

### Its size

`stdout.size()` is how many columns and rows the terminal has now, as a
`Size { cols, rows }`, or `Unsupported` if the output isn't a terminal.
It can change as the program runs, as its window does; a program that
cares asks again before it draws.

## What the checker trusts

| Intrinsic | Promise |
|---|---|
| `Clock::os_now()` | the monotonic clock in nanoseconds; never less than any it gave before |
| `Clock::os_sleep_until(deadline)` | returns only once `os_now()` is no less than `deadline` |
| `Clock::os_unix_time()` | nanoseconds since 1970, or `u64`'s greatest if before |
| `Entropy::os_fill(buf)` | writes all of `buf`, or none of what it promises, and says it failed |
| `Stdin::os_raw_enter()` | keeps how the terminal was and puts it in raw mode, or fails and changes nothing |
| `Stdin::os_raw_leave()` | puts back what `os_raw_enter` kept |
| `Stdin::os_wait(deadline)` | touches no T memory; 1 if there's input, 0 once the deadline's past |
| `Stdout::os_size()` | touches no T memory |

In C, the monotonic clock is `clock_gettime(CLOCK_MONOTONIC)`, waiting
is `nanosleep` in a loop that checks the clock again, entropy is
`getentropy` in pieces of 256 bytes, raw mode is `termios`, waiting for a
key is `poll`, and the size is `TIOCGWINSZ`. The interpreter uses Rust's
clocks, the `getrandom` crate, and the same calls, through `libc`, for `t run
--interp`; for tests, its input and output aren't a terminal.

## Design decisions worth noticing

- **Every source of nondeterminism is a value** (T1). A function that's
  handed no `Clock` or `Entropy` is one a test can call twice and get the
  same answer from. What's random is the seed; the generator is a pure
  function, in a library, as it should be.
- **Deadlines, not durations.** `sleep_until` and `wait` each take an
  `Instant`. A relative wait is the drift bug in waiting; an absolute one
  composes: many waits can share one deadline.
- **`Duration` is one `u64`** (K3): 584 years is enough, every operation
  is one machine operation, and its limits are refinements the checker
  proves, not panics.
- **Entropy can fail** (K2), as everything else in the `Os` can: a
  program that can't get a good key says so, rather than using a bad one.
- **Ctrl-C is a key** (K1): in raw mode, the program owns the keyboard,
  and the runtime's guarantee that the terminal is put back covers what
  Ctrl-C would otherwise be needed for.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `sleep(duration)` | drifts; `sleep_until(clock.now().add(d))` says what it means | — |
| calendar dates, time zones | a library, over `unix_time` | — |
| resize events (`SIGWINCH`) | ask `size()` again; events need a way to wait on several things | — |
| a terminal library: escape sequences, colours, keys parsed | a library, over `Raw` and `Stdout` | — |
| raw mode on Windows | the interpreter there says `Unsupported`; compiled programs are POSIX | — |
| a seeded random number generator in the libraries | a library, over `Entropy` | — |

## Exercises

1. Write `fn timed<F: FnOnce() -> R, R>(clock: Clock, f: F) -> (R, Duration)`.
   T has no tuples yet: what do you give back instead?
2. A server polls a feed every 15 minutes, and a fetch takes up to a
   minute. Write the loop with `sleep_until`. What does it do if a fetch
   takes 20 minutes?
3. Why can `Clock` be `Copy` but `Stdin` can't? Would a `Clock` that also
   *set* the time of day still be?
4. Write a key reader over `Raw` that turns the escape sequences of the
   four arrow keys into an enum, and everything else into `Other(u8)`.
5. Lemma T4 says a panic puts the terminal back. Where in the runtime does
   that happen, and why couldn't it be a `discard`?
6. `wait` counts bytes already in `Stdin`'s buffer as a key ready. What
   would go wrong if it only asked the operating system?

## Reading

- `termios(3)` and `poll(2)`: raw mode, and waiting on a file descriptor
  with a timeout.
- `clock_gettime(2)`: the monotonic and real-time clocks, and why they
  differ.
- `getentropy(3)`, and *Myths about /dev/urandom* (Thomas Hühn).
- Joe Duffy, *The Error Model* (2016), and the capability-based I/O of
  WASI and Pony: authority as values.
- `clock_nanosleep(2)` and its `TIMER_ABSTIME`: sleeping until a moment,
  which is what `sleep_until` is.
