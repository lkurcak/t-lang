# Step 71: what t-metal found

Code: `c_name`, `Gen::import_body`, `Gen::exports` and `Gen::run` in
`src/cgen.rs`; `Checker::extern_signature` and `Checker::externs_known`
in `src/check/externs.rs`; `hosted` in `src/lib.rs`; `escape` in
`src/lexer.rs`; `items` in `tests/docs.rs`.

t-metal is a small kernel for a 64-bit RISC-V board, run on QEMU: a shell
on the board's UART, a timer interrupt counting ticks, a key–value store
on the heap. It's the first program to use what steps 38–41 added for
machines with no operating system: the freestanding target, `extern fn`,
`Volatile`, statics and atomics. Its friction log had eight findings. This
step fixes five, and says what the language meant in a sixth:

- an `extern fn` in a module made C that didn't compile;
- a program that writes the platform's functions in T could be checked
  only by building it for the freestanding target;
- an export, which an interrupt may call, could allocate, in the middle of
  an allocation;
- the docs' blocks of items with no `fn main` were never checked, and one
  of them didn't;
- `\x` escapes were for byte literals only, so a backspace in a string
  was `\u{8}`;
- and what an import that reads C's state says.

## An `extern fn` in a module

The checker always took an `extern fn`'s own name for its C name: two
`extern fn put`s in two modules are refused, as a C name is one for the
whole program (lemma E4 of docs/40-extern.md). The C backend took its
path, `uart::put`, which isn't a C name, and the symbol it gave it was
`uart::put` too, which nothing calls. So every `extern fn` had to be at
the program's root, forwarding to the module that does the work. Now
each is C's by its own name, wherever it is, and so is a platform's
function in a module, which the backend didn't even see as the
platform's:

```rust
mod uart {
    /// The board's C function that sends a byte.
    extern fn uart_put(b: u8);

    pub fn put_text(s: &str) {
        for b in s.as_bytes().iter() {
            // SAFETY: `uart_put` only sends a byte.
            unsafe { uart_put(*b) }
        }
    }
}
```

## Checked the same for either target

A program that writes `t_platform_write` and `t_platform_halt` in T, as
docs/40-extern.md shows how to, could be checked only for the
freestanding target: for the hosted one, where `t check`, `t test` and the
language server check, writing them was an error, since the platform is
the C library's there. So the only check of t-metal's kernel was building
it, and its own modules had no tests.

That contradicted docs/39-freestanding.md: checking is the same on both
targets, and a program can be tested on the hosted one. Now it is. The
platform's functions are checked the same way for either target, their
types exactly the runtime's and their effects at most `global`, and only
running a program that writes them on the hosted target, or building it
for it, fails:

```text
$ t kernel.t
type error: this program writes the platform's `t_platform_write`, which on the hosted
            target is the C library's: it's for the freestanding target, `t build
            --target freestanding`
```

Its tests run as ever, each a hosted program of its own (docs/31-tests.md),
whose platform is the C library's. In a test, an export is a T function,
written only if the test calls it, with no C symbol, so it can't clash
with the C library's. A test can call one:

```rust
use core::sync::Atomic;

static SENT: Atomic<u32> = Atomic::new(0);

extern fn t_platform_write(_b: u8) with global {
    let _ = SENT.fetch_add(1);
}

#[test]
fn counts_bytes() with {global, panic} {
    t_platform_write(104);
    t_platform_write(105);
    assert_eq(&SENT.load(), &2);
}
```

A test that calls an import fails to link, if the C library hasn't got
that function: honestly, as the import's caller promised it exists.

## What an interrupt may do

t-metal's timer interrupt is an export, `trap`, which the board's
start-up code calls between any two instructions of `main`, including
those of the platform's `t_platform_alloc`. An allocator, as most, can't
be started again in the middle of itself, so a handler that allocates
breaks the heap. Nothing stopped one from saying `with alloc`. Now an
export's effects, said or inferred, are at most `global` and `panic`:

```rust,error
use alloc::boxed::Box;

extern fn on_interrupt(b: u8) {
    let _ = Box::new(b);
}
// error: an export can't have `alloc`
```

It's refused however the body gets `alloc`: itself, through a function,
one whose effects are inferred, a closure, a generic function, a method,
or a trait's method whose associated effect says so
(`tests/generated_exports.rs`). Freeing is no effect, but an export
frees nothing either: it's given only scalars, and a static holds only
plain data, never a box or a list, so it owns nothing on the heap.

Not every export is an interrupt's. One C calls back from a function T
called, as `qsort` calls its comparison, could allocate safely. Telling
the two apart needs a way to say which an export is; until then the rule
is the stricter one, which can be relaxed.

On the hosted target C calls an export from a signal handler only if the
program installs one, which T has no way to do yet. A panic in one would
write through the C library's `stdio`, which a signal handler mustn't.

## The docs' items, checked

`tests/docs.rs` ran every block with a `fn main`, but only parsed and
formatted one of items alone, so docs/40-extern.md's `put`, which wrote
`for b in s.as_bytes()`, had never been checked, and didn't: a slice
isn't an `Iterator`. Now a block of items is checked as a program with an
empty `fn main` after it. That its items aren't used is no warning, as
it's part of a program; any other must be in a `// warning:` comment. It
isn't run, as there's nothing to run, which is what lets docs/40's
platform functions be checked. Five blocks failed: a `Never` that wasn't
declared, a polymorphic recursion whose `T` wasn't `Discard` (it's shown
for its error now), the slice, the platform's functions, and a module's
file, `use super::Point`, which is marked `module` now, as what it names
is in the modules around it.

## `\x` in strings and characters

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"\x1b[0m".len())?;     // expect: 4
    out.line(&('\x08' as u32))?;     // expect: 8
    out.line(&"\x41\u{42}C".len())?; // expect: 3
    Ok(())
}
```

A string or a character can have `\x00` to `\x7F`, as in Rust: an ASCII
character, written by its number. Past that, `\x` is a byte that's only
part of a character in UTF-8, so it's still for byte literals alone, and
`"caf\xE9"` is refused:

```text
syntax error: `\xE9` isn't ASCII, so it's a byte, not a character; write the
              character as `\u{E9}`
```

## An import that reads C's state

docs/40-extern.md now says what t-metal's imports already did: one that
reads or writes state outliving the call, C's statics or a device's, says
`with global`, as a T function using a static does, since two calls may
give different things. It's the import's caller's promise, as all of an
import's effects are (lemma E1).

## Found along the way

The tests that rename every name in every program, and check it does the
same after, renamed `t_platform_write` in the new test of a program that
writes it: it became an ordinary export, and the program ran. An `extern
fn`'s name is its C name, so renaming one is never only a rename: an
import would be another C function, an export would no longer be what C
calls. An editor's rename refuses it now, as it refuses `main`.

## Why it's sound

- **Lemma N1 (names).** The checker proves the program's `extern fn`s'
  own names unique (E4); the backend gives each its own name as its
  symbol, so the object's symbols are those proved unique, and a
  platform's function is the platform's by the same name the checker
  checked it as one by.
- **Lemma T1 (targets).** Checking depends on the target only through the
  libraries it provides (F1 of docs/39-freestanding.md) and std's report
  of `main`'s error (F4). The platform's functions are checked by P of
  docs/40-extern.md on either target. A program run or built for the
  hosted target writes none, so the runtime there calls the C library's,
  as it always has. A test's program defines no C symbol of an export's,
  so nothing of the C library's is defined twice, and its exports'
  callers are T code, checked, so E2 holds of them trivially.
- **Lemma E2 (exports, restated).** What an export shares with the code
  C interrupts to call it is statics, which are `Sync` and change only
  atomically (Y1, Y3 of docs/38-statics.md), and the heap. Effects bound
  what a body does (docs/07-effects.md), and its effects are at most
  `global` and `panic`, so it never allocates. Freeing needs a value that
  owns heap memory, and an export has none: its parameters are scalars
  (E3), statics are plain data (docs/38-statics.md), and what it makes
  without allocating owns none. So it never enters the allocator.
- **Lemma X1 (escapes).** An ASCII byte is a whole character in UTF-8, so
  a string with `\x00`–`\x7F` in it is still UTF-8, and a character one
  is a Unicode scalar value, as each literal's type promises.

The adversarial tests are `tests/generated_exports.rs` (64 programs:
an export or the platform's write, × an effect, × a way to get it, each
checked for both targets, which must agree), `tests/t/err_extern_hook_run_hosted.t`,
`tests/t/err_str_escape_x.t` and `err_char_escape_x.t`, and
`tests/extern`'s `hosted_modules.t` and `machine_modules.t`, built and
run.

## What it doesn't do

- `t check` doesn't refuse a program that writes the platform and uses
  std, which no target can build: whether a program uses std isn't
  something checking knows, as which libraries a package without a
  `t.toml` has is its target's. Building it says so.
- `for` over a slice still needs `.iter()`: whether a slice is
  `IntoIterator` is an open question since step 9.1.
- From the same log, left for later: `/` and `%` by a constant in
  predicates and loop invariants, hexadecimal formatting, `&array[..]` as a
  slice, and a string continued on the next line.

## Decisions

- D2, checking is the same for both targets; a hosted run or build
  refuses a program writing the platform; a test's exports aren't C's.
  The alternatives, a `target` in `t.toml` or `--target` on `check`,
  `test` and the language server, would make a program's checking depend
  on how it's built.
- D3, an export's effects are at most `global` and `panic`, for every
  export, until one can say it's no interrupt's.
- D4, `\x00`–`\x7F` in strings and characters, as in Rust.
- Not refusing the platform's functions with std at checking.

## Exercises

1. Why is it enough to look at an export's effects to know it never
   enters the allocator? Find what would break that if statics could hold
   a `Box`.
2. A `qsort` comparison written in T is an export that C calls only from
   inside a call T made. Design a way to say so that lets it allocate,
   and say what the checker must check of the import that calls it back.
3. Why is `\xE9` fine in `b"caf\xE9"` but not in `"caf\xE9"`? What would
   `"caf\xC3\xA9"` be, if `\x` could be any byte, and why is that still
   a bad idea?
4. A test of a program that writes `t_platform_halt` panics. What stops
   it: the program's `t_platform_halt`, or the C library's? Why is that
   the right answer?

## Reading

- The Rust reference, *Tokens*: character and string escapes, and why
  `\x` stops at `\x7F` outside byte literals.
- The Linux man page *signal-safety(7)*: what a signal handler may call,
  and why `malloc` and `printf` aren't on the list.
- John Regehr, *Safe and Structured Use of Interrupts in Real-Time and
  Embedded Software* (2007): what an interrupt handler shares with the
  code it interrupts.
