# Step 31: tests

Code: `parser.rs` (`Head::test`, `no_test`), `check.rs`
(`Checker::test_signature`, `entry_result`, `tests_known`), `check/lints.rs`
(`dead_code`: tests are live), `mono.rs` (`Executable::report`), in
`interp.rs`, `run_entry` and `Interp::message`, `Error::blamed_caller` in
`error.rs`, `test_files` in `lib.rs`, and `core/panic.t`.

Until now a T program was tested by running it and comparing what it
printed with a file: tcalc and the hash map each have a `tests/run.sh` that
does that, and the hash map's `// 36` comments, saying what each line
should be, were checked by nothing but the diff. Now a function can be a
test:

```rust
fn double(x: i64) -> i64 { x * 2 }

fn main() {}

#[test]
fn doubles() {
    assert_eq(&double(3), &6);
    assert(double(0) == 0);
}

mod parsing {
    #[test]
    fn fails() {
        assert_eq(&crate::double(1), &3);
    }
}

// test: doubles ... ok
// test: parsing::fails ... FAILED: assertion `left == right` failed left: 2 right: 3
```

and `t test main.t` runs each, in the order they're written:

```text
test doubles ... ok
test parsing::fails ... FAILED

--- parsing::fails
runtime error: panicked: assertion `left == right` failed
  left: 2
 right: 3
 --> main.t:14:9
   |
14 |         assert_eq(&crate::double(1), &3);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1 passed; 1 failed
```

It fails if any test does, or if the program doesn't check. `t test
main.t pars` runs only the tests whose names have `pars` in them. `t test
lib.t` runs a library's: a library has no `main`, and needs none to be
tested.

## A test is a program

What running a test means is what running a program means: a test `f` is
run as the program

```text
fn main() -> R { f() }
```

would be. So it starts from nothing, with memory of its own, and ends as a
program does: it passes if it gives back `()` or `Ok(())`, and fails if it
panics, runs out of memory or of stack, or gives back an `Err`, which is
reported by its `Display` as `main`'s is. Everything the interpreter checks
of a run, it checks of each test's: that what it does is in its effects,
that refinements hold, that everything was freed.

Its signature follows: a test is what `main` can be, but for the `Os`.

- It takes nothing, and has no generic parameters: nothing could give it
  an argument or say what a type parameter is.
- It gives back `()` or a `Result<(), E>` whose `E` is `Display` and
  `Discard`, as `main`.
- It can't say `with unsafe`: its obligations would be its caller's to
  keep, and nothing calls it.
- It's a function of a module: not a method (nothing could give it its
  `self`), and not inside another function.
- It's neither `pub` nor `export`: `t test` runs it, and nothing else needs
  to see it.

Its effects are inferred, as any private function's are (step 24): a test
says no `with`.

```rust
fn main() {}

#[test]
fn takes(x: u8) {
    assert(x == 1);
}
// error: a test takes nothing
```

```text
impl S {
    #[test]
    fn m() {}
}

error: `#[test]` can only be on a function of a module, not a method or a
       function inside another
```

A test that gives back a `Result` can use `?`:

```rust
use alloc::string::String;

fn main() {}

fn parse(s: &str) -> Result<u8, String> {
    match s.len() {
        1 => Result::Ok(1),
        _ => Result::Err(String::from("not one byte")),
    }
}

#[test]
fn fails_to_parse() -> Result<(), String> {
    let _ = parse("ab")?;
    Result::Ok(())
}

// test: fails_to_parse ... FAILED: gave back an error: not one byte
```

## No I/O, and so nothing shared

`main` is given the `Os`, and every stream comes from it (step 19). A test
is given nothing. So a test can't read, write or print: what it does is
all in what it gives back, or why it stopped. That, and that T has no
global state, makes tests independent by construction. None can see
another's memory, or anything another left behind; running them in
another order, or only some, or each on a machine of its own, gives each
the same result.

```rust
fn main() {}

#[test]
fn prints(os: std::Os) {
    let mut out = os.stdout;
    let _ = out.line(&1u8);
}
// error: a test takes nothing
```

It's also why a test that panics can fail without taking the others with
it, though T has no way to catch a panic, and won't (docs/07-effects.md:
a panic aborts, which lemma P4 of step 30 relies on). Nothing needs to be
caught: each test is a run of its own, so a panic aborts only that run.
The interpreter starts each afresh; a compiled `t test` would run each in
a process of its own, or start again after a panic, as `fork` lets it.

## Tests that panic

`#[test(panics)]` is a test that passes only by panicking. Not by
finishing, nor by giving back an `Err`, nor by running out of memory or of
stack, which are aborts but not panics: memory is `alloc`'s (step 16), and
running out of stack isn't any effect's.

```rust
fn main() {}

fn first(xs: &[u8]) -> u8 { *xs.get(0).unwrap() }

#[test(panics)]
fn of_nothing() {
    let xs: [u8; 0] = [];
    let _ = first(xs.as_slice());
}

#[test(panics)]
fn of_something() {
    let _ = first([7u8].as_slice());
}

// test: of_nothing ... ok
// test: of_something ... FAILED: didn't panic, which this test passes only by doing
```

One whose effects have no `panic` is an error, as it could never pass:

```rust
fn main() {}

#[test(panics)]
fn never() {
    let _ = 1u8;
}
// error: this test passes only if it panics, and it can't
```

## Assertions

The prelude has three, from `core::panic`:

- `assert(ok)` panics, saying `assertion failed`, unless `ok`;
- `assert_eq(&left, &right)` panics unless `left == right`, saying what
  each is;
- `assert_ne(&left, &right)` panics unless they're different.

`assert_eq` needs its values `Eq`, to compare them, and `Display`, to say
them. Saying them is a message built as the program stops, and the
message of `panic` is a `&str`. Formatting into a `String` would need
`alloc`, and an assertion would allocate where the code it checks
doesn't. So `core::panic::panic_with(&why)` panics saying any `Display`,
written into a sink only it can make, `core::panic::Message`, which can't
fail (`type Error = !`) and does nothing (`effect e = {}`). `Display::fmt`
does only what its sink does, so writing the message can't fail, panic or
allocate: `assert_eq` is `with panic` and nothing else.

```rust
fn main() { core::panic::panic_with(&42i64) }
// error: runtime error: panicked: 42
```

The sink isn't I/O behind `Os`'s back: what's written to it goes nowhere
the program can see, and nothing runs after it but the end of the
program. `Message` isn't exported, and the only function that makes one
aborts with it.

## Where it failed

An assertion that fails panics in the core library, in `assert_eq`'s
body: showing that would show nothing. So a panic that happens in a
built-in library, core, alloc or std, is shown at the innermost call that
isn't in one: the line of the `assert_eq`, or of the `unwrap`. That's for
every run, not only tests:

```rust
fn main() {
    let x: Option<u8> = Option::None;
    let _ = x.unwrap(); // shown here, not in core/option.t
}
// error: panicked: `unwrap` of `None`
```

It's what Rust's `#[track_caller]` does, but for every function of the
built-in libraries, and with no attribute: nothing in them is the
program's to fix. A dependency's code isn't skipped: a panic in a library
someone wrote is shown there, where its bug is.

## One meaning: no `cfg`

Tests are part of the program. `t check` checks them, as it checks
everything; `t` doesn't run them, and `t test` doesn't run `main`. There's
no `#[cfg(test)]`, and so nothing in a program means something under `t
test` that it doesn't under `t check`.

A test is live, as `main` and what's exported are, so what only tests use
isn't dead code:

```rust
fn main() {}

fn helper() -> u8 { 1 }

fn unused() -> u8 { 2 } // warning: function `unused` is never used

#[test]
fn uses_helper() {
    assert_eq(&helper(), &1);
}

// test: uses_helper ... ok
```

A dependency's tests are checked with it, as everything of it is, but only
the program's own are run: they're what the program says of itself.

## Warnings

`--deny-warnings`, after `t check` or `t test`, makes any warning a
failure. The tests still run, and their results are still shown, so a
warning while working on something doesn't hide whether it works. CI
passes it, where tcalc's `run.sh` used to look for what `t check` printed.

## Why it's sound

- **T1, one meaning.** `#[test]` adds no code and changes none: it makes a
  function a root, of what's live and of what `t test` runs. So a package
  checks the same under every command, and `t check` accepting it means
  its tests are well-typed too.
- **T2, a test is a program.** A test is run exactly as `fn main() -> R {
  f() }` would be, so everything shown of programs holds of it: its
  signature is one `main` could have but for the `Os`, which only makes it
  do less. Each run starts from nothing and ends with the leak check, so a
  test can't be passed or failed by what another did; with no `Os` and no
  globals, nothing outside the run can tell tests apart from one another
  but by what they give back.
- **T3, `panics`.** A `#[test(panics)]` passes only by the `panic` effect
  happening, which the interpreter tells apart from running out of memory
  or stack. One whose effects lack `panic` can't pass, by the effect
  system's soundness (docs/07-effects.md), so it's an error.
- **T4, the message sink.** `panic_with` writes into a `Message`, which
  only it can make and which aborts the program; `Display::fmt` into it is
  total, as it does only what `Message` does, which is nothing. So it adds
  no I/O, no effect but `panic`, and no way to keep running after.
- **T5, blaming the caller.** Where a panic is shown changes no result:
  it's a choice of span, among the calls the panic happened in.

## Design decisions worth noticing

- **No `#[cfg(test)]`.** Rust compiles tests only under `cargo test`, so a
  test can use what a normal build doesn't have, and a program can mean
  two things. Here one rule, tests are roots, gets the useful part
  (helpers used only by tests aren't dead) without the second meaning.
- **No unwinding, no `catch_unwind`.** Rust's test harness catches panics,
  which is why a Rust panic unwinds by default. T's tests are runs, so
  panics can go on aborting, and step 30's invariants can go on relying on
  it.
- **No `Os` for tests.** The tests that check what a program prints, as
  tcalc's transcript does, are still a script's. They'd need a way to give
  a test streams in memory; that's a design of its own, for when it's
  needed.
- **`assert_eq` takes references**, as T has no macros: `assert_eq(&a,
  &b)`. It doesn't need `Copy`, and says the values by `Display`.
- **Caller blame without `#[track_caller]`.** An attribute would be one
  more thing for a library author to remember on every function that
  checks its arguments; drawing the line at the built-in libraries needs
  none.

## Deliberately missing (coming later)

- Tests of what a program prints, given what it reads.
- Doc tests: the code blocks of `///` comments, as these docs' blocks are
  checked.
- Running tests in parallel, and a time limit for each.
- `assert` that tells the refinement checker what it checked: after
  `assert(i < xs.len())`, the fact `i < xs.len()`.
- Benchmarks.

## Exercises

1. Write a test that passes when run alone but would fail after another
   test, in a language with globals. Why can't you write it in T?
2. Make `assert_eq` need `Copy` instead of taking references. Which of
   the core library's types could it then compare?
3. Turn off the rule that a test takes nothing, and give a test the `Os`.
   What does T2 stop saying, and what goes wrong with running tests in
   another order?
4. `#[test(panics)]` fails on running out of memory. Give an example
   where passing on it would hide a bug.
5. Write `assert` for refinements: what would its signature need to say
   so that `xs[i]` checks after `assert(i < xs.len())`? Compare `ensures`
   (step 30).

## Reading

- Kent Beck, *Simple Smalltalk Testing: With Patterns* (1994), where the
  xUnit shape, tests as functions run one by one, comes from.
- The Rust Reference, *Testing attributes* (`#[test]`, `#[should_panic]`)
  and RFC 2091, *Implicit caller location* (`#[track_caller]`).
- Koen Claessen and John Hughes, *QuickCheck: A Lightweight Tool for
  Random Testing of Haskell Programs* (2000): what tests can be when they
  are pure functions, as T's are.
- Andrew Baumann et al., *A fork() in the road* (2019): what `fork` costs,
  for running each test as a process of its own.
