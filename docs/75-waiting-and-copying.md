# Step 75: waiting and copying

Code: `spin` and `Barrier` in `core/sync.t`; `Threads::available` in
`std/thread.t`; `copy_from_slice` in `core/slice.t`, `RawBuf::copy_in` in
`alloc/raw.t`, and `List::extend_from_slice` and `List::from_slice` in
`alloc/list.t`; the intrinsics in `src/intrinsic.rs`, run by
`Interp::slice`, `Interp::raw_buf` and `Interp::call_intrinsic` in
`src/interp.rs`, and written out in `src/cgen.rs` and
`src/cgen/hosted.h` (`t_spin`, `t_threads_available`).

t-life, the first program on several threads (docs/41-threads.md), had
to build what it needed to wait for its threads, out of atomics, and to
copy cells one at a time:

- A thread could wait for another only by looking again and again, with
  nothing to tell the machine it was waiting: `core::sync::spin`, which
  `Mutex::lock` gives it, was private. So t-life's barrier kept a core
  busy, and slowed its neighbour on the core's other hardware thread.
- Nothing said how many threads the machine runs at once, so `--threads`
  was 4 unless the command line said.
- A barrier, where threads wait for each other, it wrote itself.
- Nothing copied a slice: a row of cells was copied a cell at a time, and
  a list made a `push` at a time, about 7 ns a cell, three times as long
  as working the cell's next generation out.

Now:

```rust
use alloc::list::List;
use core::sync::{Atomic, Barrier, spin};

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    // At least 1: the refinement says so.
    let n: { n: usize | n >= 1 } = os.threads.available();
    let barrier = Barrier::new(2);
    let ready = Atomic::new(false);
    let h = os.threads.spawn(|| {
        ready.store(true);
        barrier.wait()
    });
    while !ready.load() {
        spin();
    }
    let me = barrier.wait();
    let it = h.join();
    // One of the two came last.
    out.line(&(me != it))?; // expect: true

    let mut row = [0u8; 4];
    let cells = List::filled(4, 1u8);
    row.as_mut_slice().copy_from_slice(cells.as_slice());
    let mut more = List::from_slice(row.as_slice());
    more.extend_from_slice(cells.as_slice());
    let last = more[7];
    out.line(&f"{n >= 1} {more.len()} {last}") // expect: true 8 1
}
```

## `spin`

`core::sync::spin()` says the thread is waiting for another, in a loop
that looks again and again until that one has done something. It changes
nothing: what it does is the machine's business. On the hosted target it
gives the thread's time up to the operating system (`sched_yield`), so
that a thread it waits for that isn't running gets to run. On the
freestanding target, with no operating system, it tells the processor the
thread is waiting (`pause` on x86-64, `yield` on AArch64). In the
interpreter, it hands the baton to the next thread (docs/41-threads.md,
I), as each atomic operation does.

Which the hosted target does was measured, on t-life's pool of threads,
which waits at a barrier twice a generation (Ryzen 9 7950X: 16 cores, 32
hardware threads; milliseconds for the pool, median of three):

| board, generations | threads | no hint | `pause` | `sched_yield` | `pause`, 1 in 16 `sched_yield` |
|---|---:|---:|---:|---:|---:|
| 2048², 200 | 16 | 321 | 288 | 317 | 321 |
| 2048², 200 | 32 | 477 | 473 | 338 | 346 |
| 2048², 200 | 64 | 1025 | 985 | 379 | 369 |
| 512², 300 | 16 | 31 | 31 | 30 | 31 |
| 512², 300 | 32 | 486 | 487 | 34 | 35 |
| 512², 300 | 64 | 1090 | 1175 | 40 | 41 |

With a thread waiting for each hardware thread, or more, a waiting thread
that doesn't give its time up holds a core that the thread it waits for
needs, and the pool is up to twenty-seven times slower. With fewer,
giving it up costs nothing that shows. Telling the processor only, as
Rust's `spin_loop` does, is the freestanding target's, where there's no
one to give the time up to.

It has no effects: a `with {}` function can wait.

## How many threads at once

`os.threads.available()` is how many threads the machine runs at once,
for the program: on Linux, the processors it may run on, as `taskset` and
containers limit them; elsewhere, those online. It's
`{ n: usize | n >= 1 }`: a count of threads to start, or of bands to cut
a board into, that's never 0, without a check.

It's a method of `Threads`, the right to start threads, which a program
needs to use the number anyway. It's what the machine says, so it may be
different in the interpreter and in the compiled program, as the clock
is, and from one run to the next.

## `Barrier`

`core::sync::Barrier::new(count)` is where `count` threads wait for each
other: each calls `wait`, and none goes on until all have. Then the next
round starts, so it can be used again and again: each generation, in
t-life. `wait` gives `true` to one thread a round, the last to come, for
work one thread does between rounds.

It's safe T, on two atomics: how many have come, and which round it is.

```rust
use core::sync::Atomic;

#[derive(Discard, Send, Sync)]
pub struct Barrier {
    count: { n: usize | n >= 1 },
    here: Atomic<usize>,
    round: Atomic<usize>,
}

impl Barrier {
    pub fn wait(&self) -> bool {
        let round = self.round.load();
        if self.here.fetch_add(1) == self.count - 1 {
            self.here.store(0);
            let _ = self.round.fetch_add(1);
            true
        } else {
            while self.round.load() == round {
                core::sync::spin();
            }
            false
        }
    }
}
```

It waits by spinning, so it's for threads that each have a core, working
in short rounds. A thread that waits long would better sleep, which needs
the operating system's help: a `Condvar`, later.

## Copying slices

`s.copy_from_slice(src)` copies the values of `src` into `s`, all at once,
as the machine copies memory. Three things make it one copy:

- **The same length.** `src` is `{ s: &[T] | s.len() == self.len() }`,
  which the checker proves, as it does an index: there's no copying of
  "as many as both have", and no panic.
- **`T: Copy`.** What's copied stays in `src`, and what was in `s` is
  `Copy` too, so dropping it does nothing (`Copy` is only derived, and a
  `Copy` type has no `discard` of its own): nothing linear is lost, or
  had twice.
- **Apart.** `s` is lent `&mut` while `src` is a `&` alive at the same
  time, so they share no values (the borrow checker's lemma), and a copy
  of memory that mustn't overlap is right. A slice can't be copied into
  itself, nor a list's values into it through `as_slice`.

`List::extend_from_slice(s)` puts copies of `s`'s values at a list's end,
growing it once if it must, and `ensures self.len() == old(self.len()) +
s.len()`. `List::from_slice(s)` is a new list of them, as long as `s`.
Both are safe T over one new intrinsic of `RawBuf`: `copy_in(at, src)`,
unsafe as `write` is, as its slots must hold nothing (the interpreter
checks they don't). `core::slice::empty()` and `empty_mut()` now say their
length is 0, so `s.copy_from_slice(empty())` of an empty `s` checks.

A slice of refined values keeps its refinement: `T` is instantiated at
the elements' refined type, as `Copy` is a marker
(docs/60-refined-type-arguments.md), so copying a slice of `u8`s into
one of digits needs the `u8`s to be digits.

## Why it's sound

- **`spin` is nothing.** Its meaning is to do nothing; what it does on a
  machine, a pause or a turn given up, is unseen by the program, and
  the interpreter's handover is a schedule a machine could have chosen.
  A busy loop could already do everything a loop with `spin` does.
- **The count is proved.** Only the intrinsic's type, a `usize`, is
  trusted of what the machine says; that it's at least 1 is safe code in
  `std/thread.t`, which makes 0 into 1.
- **A barrier is safe code.** It's two atomics, and `Sync` is derived
  from its fields: no `unsafe`, so no data race (lemma Z2 of
  docs/41-threads.md). That it works: `round` changes only when the last
  thread comes, after it set `here` back to 0, and the atomics are
  sequentially consistent, so a thread that sees the new round, and comes
  again, adds to `here` after it's 0. A thread waiting for round `r`
  can't miss it for `r + 2^64`: the round can't change again until that
  thread comes. More threads than `count`, or one that never comes, hang
  the rest, which is a bug of the program's but no breach of a type.
- **Copies are in bounds and apart.** The refinements put the copy in
  range, which the interpreter checks; the borrows make the two apart,
  which the interpreter checks too (a copy between slices sharing values
  is "type soundness violated"). Of `Copy` values, a copy is what a `let`
  would do, value by value, and each value written is one of `src`'s, of
  type `T`: what's known of `T` holds of it (lemma A2 of docs/60).
- **`copy_in` is an obligation.** Like `write`, it's unsafe because the
  slots it fills must be empty, which only the code using the buffer can
  know. `extend_from_slice` keeps `List`'s invariant: the slots it fills
  are past `len`, and `len` counts them after.

The adversarial tests are `tests/t/err_soundness_copy_from_slice_*.t`,
`err_soundness_extend_from_*.t`, `err_soundness_rawbuf_copy_in_safe.t`,
`err_unsafe_rawbuf_copy_in_init.t` and `err_barrier_zero.t`.

## What it doesn't do

- **Sleeping.** A thread that waits spins: there's no `Condvar`, no
  parking, and no sleeping barrier, which need the operating system's
  futexes (or the like) in the runtime.
- **Copies of values that aren't `Copy`.** A `clone_from_slice` needs a
  `Clone`, which T doesn't have.
- **Overlapping copies.** Within one slice, as Rust's `copy_within` does:
  `split_at_mut` and `copy_from_slice` cover the copies that don't
  overlap.
- **Weaker orderings.** The barrier's atomics are sequentially
  consistent, as all of T's are (docs/38-statics.md).

## Decisions

- **D1: one `spin`.** Not Rust's two, a hint to the processor and a turn
  given up to the operating system: the target picks what a wait is.
- **D2: `os.threads.available()`.** A method of the right to start
  threads, with no effects.
- **D3: `Barrier` in `core::sync`, spinning, `wait() -> bool`.** True for
  the last to come, which needs no `#[must_use]`: only a `Result` warns
  when it's dropped.
- **D4: Rust's names, one copy of memory.** `copy_from_slice`,
  `extend_from_slice`, `from_slice`, and `copy_in` under them.

## Exercises

1. Write `Barrier::wait` with one atomic, counting up for ever: round
   `r` is over when it's `count * (r + 1)`. What goes wrong when it wraps?
2. Why must `wait` read `round` before it adds to `here`, not after?
3. `copy_from_slice` needs `T: Copy`. Write the program that would free a
   list twice if it didn't.
4. Write `copy_within(&mut self, from: Range<usize>, to: usize)` in safe T,
   with `split_at_mut` and `copy_from_slice`, for ranges that don't
   overlap. What would one that may overlap need?
5. `extend_from_slice` grows the buffer to twice what it was, or to fit.
   Why not just to fit?

## Reading

- Herlihy and Shavit, *The Art of Multiprocessor Programming* (2008),
  chapter 17: barriers, the sense-reversing barrier this one is like.
- Intel, *64 and IA-32 Architectures Software Developer's Manual*,
  `PAUSE`: what it tells the processor, and why spin loops should say it.
- The Linux manual, `sched_yield(2)` and `sched_getaffinity(2)`.
- The Rust standard library: `std::hint::spin_loop`,
  `std::thread::available_parallelism`, `std::sync::Barrier`, and
  `slice::copy_from_slice`.
