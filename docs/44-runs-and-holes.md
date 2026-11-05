# Step 44: compiled runs, expressions in holes, `#[warn]`

Code: `main.rs` (`run_compiled`, `--interp`, `PANICKED`), `test_native`
and `run_test` in `src/lib.rs`, `emit_tests` and `Gen::test_main` in
`src/cgen.rs`, `t_begin_panic` in `src/cgen/shim.h`, and the hosted
`t_platform_halt`; `fstring_hole` and `lex_between` in `src/lexer.rs`,
`Parser::hole` and `Parser::inner_allows` in `src/parser.rs`, the hole
lints in `Rewrite::expr` (`src/check/modules.rs`) and
`Checker::hole_effects` (`src/check/effects.rs`); `src/allow.rs`.

Retrospective 4 found three small things worth doing at once. The
interpreter was what ran a program, and it's about a thousand times
slower than the program compiled: t-nonograms solved its puzzles in 11
seconds interpreted and in under 10 milliseconds built. A format
string's hole could hold only a name or a field, so `f"row {y + 1}"`
needed a `let` first. And lints were all on, so a lint that says what
someone may *want* of a program, rather than what's likely a mistake in
it, had nowhere to go.

## Compiled by default

`t main.t` and `t run main.t` now do what `t build` does, into a
directory of their own that's removed after, then run the program with
`t`'s own input and output, and end as it ends. `t test` writes one C
program holding every test, and runs it once for each, each as a
process of its own: what docs/31-tests.md says a test is, a program,
now literally. `--interp` runs either in the interpreter instead:

```text
$ t test main.t            # compiled
$ t test --interp main.t   # interpreted, checking as it goes
$ t --interp main.t
```

Both say the same: a panic's message, where it's shown, what was
written before it, and how a test went. That isn't a hope but a test:
`tests/compiled.rs` runs every program of the test suite, the examples
and these docs both ways and compares what they write and how they end,
and does the same for every program's tests.

> **Lemma N1 (the same program).** A program that checks does the same
> compiled as interpreted, up to the limits of the machine: the
> interpreter's on memory and on how deep calls go, the C compiler's
> stack. What the interpreter checks besides (that every site's effects
> are its function's, that refinements hold, that everything is freed)
> are checks of the *checker*, which proved them; a program needs them
> no more compiled than it does interpreted. Only `unsafe` code can
> break what was proved, and what it does then is undefined compiled,
> as `unsafe` says.

> **Lemma N2 (tests are programs).** Each test runs as a process of its
> own, from nothing: no memory, statics or streams are shared between
> two, so no test can make another pass or fail.

A panic now ends a program with status **101**, compiled or
interpreted, as in Rust; running out of memory, giving back an `Err`
from `main`, and failing to flush output end with 1. So a script can
tell a bug from a failure, and `t test` tells a `#[test(panics)]` that
panicked from one that ran out of memory, which it must not pass:

| Ends by | Status |
|---|---|
| returning `()` or `Ok` | 0 |
| returning `Err`, or output that can't be flushed | 1 |
| a panic: `panic`, an assertion, overflow, division by zero, a failed `as` | 101 |
| running out of memory | 1 |
| running out of stack (compiled) | a signal, which `t` says |

The C program marks a panic as it starts saying why it stops
(`t_begin_panic`), and the hosted platform's `t_platform_halt` ends with
101 if it's marked. A freestanding platform halts as it halts.

With no C compiler, `t main.t` fails, saying there's none and that
`--interp` needs none; it doesn't fall back to the interpreter by
itself, since a program a thousand times slower than expected is a
surprise of its own.

## Expressions in holes

A hole now holds any expression:

```rust
#[derive(Debug, Discard)]
struct Point { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Point { x: 3, y: -4 };
    let xs = [10i64, 20, 30];
    out.line(&f"{p.x + 1} {xs[1]} {xs.len()} {Point { x: p.y, y: p.x }:?}")?;
    Result::Ok(())
}
// expect: 4 20 3 Point { x: -4, y: 3 }
```

Lemma F1 of step 43.1 said a hole holds a place, so that nothing in it
runs. Its reason was that a hole couldn't hide a panic; but any
argument of any call can hold a call that panics, and the checker sees
it there as it would anywhere: effects are checked, or inferred, by
what's written, wherever it's written.

> **Lemma F1′ (what a hole is).** A hole holds an expression `e`, and
> means `core::fmt::display(&e)`, or `debug(&e)`: an argument like any.
> So everything about it is checked as it is for any argument: its
> effects are its function's, a value it moves is moved, what it borrows
> is borrowed for as long as the string is used, and if `e` isn't a
> place, `&e` borrows a temporary, which is dropped at the end of the
> statement, as for `&f(x)` anywhere.

Lemmas F2 and F3 hold as they were: what the string means is still the
three library functions, and writing it still does only what the sink
does. What a hole works out is worked out where the string is made,
not where it's written.

Where an expression ends: at the first `}`, or `:`, that isn't inside
brackets, braces or parentheses of the hole's own, nor part of a `::`.
A `:` starts what's said of how it's written, which is still only `?`.
A character literal is the expression's, so `{if a { '}' } else { '{' }}`
is one hole. A hole can't hold a `"`, nor a comment: a tool that reads
the format string without knowing about holes, an editor's highlighter
say, would see it end there.

```text
out.line(&f"{"x".len()}")?;
// syntax error: a hole in a format string can't hold a `"`; work out what needs one in a `let` first
```

A string that outlives what a hole made is caught as any borrow of a
temporary is:

```rust,error
use alloc::string::String;

fn make() -> String with alloc { String::of(&"temporary") }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let line = f"<{make()}>";
    out.line(&line)?;
    Result::Ok(())
}
// error: a temporary value is dropped here while still borrowed
```

## Lints that are off: `#[warn(..)]` and `#![..]`

Some programs do want what step 43.1 had: holes that run nothing, so
reading a format string never means reading a call. That's a choice of
style, not a likely mistake, so it's a lint that's *off* unless asked
for. Two of them:

- `format_hole_expressions` warns of a hole that holds more than a name
  or a field of one;
- `format_hole_effects` warns of anything in a hole that may do more
  than give its value: panic, allocate, use a static, or anything an
  effect parameter stands for. It's worked out once every function's
  effects are known, inferred ones included, and an operator proved not
  to overflow has none.

`#[warn(..)]` turns lints on wherever `#[allow(..)]` turns them off: on
a function, a type, a field, a variant or a `mod`. For a whole module,
or the whole program, `#![warn(..)]` and `#![allow(..)]` go at the top
of the module, after its `//!` doc comment: of its file, or of the
`{ }` of `mod m { .. }`. At the top of the crate root, they're of all of
the program.

```rust
#![warn(format_hole_effects)]

fn half(n: i64) -> i64 {
    if n < 0 {
        panic("negative");
    }
    n / 2
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let n = 7i64;
    out.line(&f"{n / 2} {half(n)}")?;
    Result::Ok(())
}
// expect: 3 3
// warning: `half` may panic in a hole of a format string; work it out in a `let` first
```

Where several attributes say something of a lint, the innermost
decides: a function's over its module's, a module's own `#![..]` over
the `#[..]` on its `mod` (the module is inside its declaration), and of
two on the same thing, the last. Where none does, the lint is on unless
it's one of the two above. `--deny-warnings` fails on what's on.

> **Lemma W1 (lints don't mean).** Turning a lint on or off changes
> which warnings are shown, and nothing else: no lint is part of
> checking, so a program means the same whatever its attributes say.

## Small things

- "has no method in scope" names every trait that has the method, with
  an import for each, rather than the first: `n.fmt(w)` could mean
  `Display`'s or `Debug`'s, and which is the programmer's to say.
- A stray `a.out` from step 29 is out of the repository.
- Compiling tests found a bug of the C backend's: the struct of an
  array of `&str`s could be written before the struct of `&str` it
  holds, as the array's element was taken for what it holds without
  looking through the reference (`held` in `src/cgen.rs`). `t build`
  of each program happened to meet the types in an order that hid it.

## Design decisions worth noticing

- **Compiled by default, no fallback.** The interpreter is the oracle:
  it checks what the checker proved, at every step, which is what T's
  own tests want, and what a user wants once in a while, with
  `--interp`. What a user wants every time is their program, at its
  speed. With no C compiler `t` says so rather than quietly running a
  thousand times slower.
- **`-O2`, as `t build`.** `t main.t` means `t build` and run, exactly:
  one compiled program, not a debug one and a release one. On
  t-nonograms (19,000 lines of C) that's 0.9 seconds of compiling,
  against 0.4 at `-O0`.
- **101 for a panic.** Rust's convention, so that tools that know it
  already know T's.
- **Holes are arguments.** F1 was a restriction without a soundness
  reason: nothing about a hole needed it. Lifting it costs no rule, and
  the restriction is still there for whoever wants it, as a lint.
- **Off-by-default lints rather than a second kind of check.** A
  program's style is its own; the language says what's sound.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| a cache of compiled programs | compiling is under a second for programs this size | — |
| arguments to the program, `t main.t a b` | `Os` has no arguments yet | Os design pass |
| `"` and comments in holes | tools that don't know holes would misread them | — |
| `{n:x}`, `{n:>8}`, `{x:#?}` | format options need a design | — |
| `#![deny(..)]`, lint groups | `--deny-warnings` covers what's asked so far | — |
| compiled `t test` in parallel | each runs in milliseconds; order is kept | — |

## Exercises

1. `f"{x} {bump(&mut x)}"` is an error. Which borrow is in the way,
   and how long does it last? Would it be one if the holes were the
   other way round?
2. Find a program for which `format_hole_effects` warns but
   `format_hole_expressions` doesn't, and one for the other way round.
3. Why does `t test` pass a `#[test(panics)]` that exits with 101 but
   not one that exits with 1? What would go wrong with a test of a
   function that runs out of memory if it passed both?
4. Lemma N1 says the interpreter's extra checks are of the checker.
   Which of them would still catch something in a program that uses
   `unsafe`, and why can't the compiled program?
5. A hole ends at the first top-level `:`. Find an expression that has
   one, and say why T has no other.

## Reading

- Python's PEP 498 and PEP 701, *Literal String Interpolation* and its
  syntactic formalization: any expression in a hole, and, in 3.12,
  quotes too.
- The Rust compiler's lint levels, `allow`, `warn`, `deny` and
  `forbid`, and how the innermost attribute decides.
- McKeeman, *Differential Testing for Software*: two implementations
  checked against each other, as the interpreter and the C backend are.
