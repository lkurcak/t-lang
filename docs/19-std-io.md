# Step 19: std I/O

Code: `std/lib.t` (`Os`), `std/io.t` (the streams, `Error`, `read_line`),
`std/rt.t` (reporting `main`'s error), `src/check.rs`
(`Checker::main_signature`), `src/interp.rs` (`Terminal`, the streams'
intrinsics), `src/lib.rs` (`Process`, the streams of `t run`) and
`src/mono.rs` (the copy of `std::rt::report`).

Until now a program printed with three built-ins, `print`, `print_bool`
and `print_str`, which any function could call, whatever its type said:
`fn pure() { print_str("hi") }` checked. That was the first finding of the
first retrospective (F2, docs/17-retrospective-1.md). Now a program is
given the world it can touch, as a value:

```rust
use alloc::string::String;
use std::io::Error;
use std::Os;

fn main(os: Os) -> Result<(), Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let mut line = String::new();
    while input.read_line(&mut line)? {
        out.put(&"> ")?;
        out.line(&line.as_str())?;
    }
    Result::Ok(())
}
// stdin: hello
// stdin: wörld
// expect: > hello
// expect: > wörld
```

## A program is handed its streams

`std` is a library beside `core` and `alloc`, loaded for every program
and, like them, checked once (docs/14-packages.md). It isn't in the
prelude: a program says `use std::Os;`.

`main` takes nothing, or one `std::Os`:

```text
#[derive(Discard)]
pub struct Os { pub stdin: Stdin, pub stdout: Stdout, pub stderr: Stderr }
```

`Stdin`, `Stdout` and `Stderr` are intrinsic types, as `RawBuf` is
(docs/09-unsafe.md): they have no fields a program could write, so no
literal of one checks, and no function gives back a new one. They're
`Discard`, as there's nothing to close (the operating system closes them
when the program ends), but not `Copy`. So there's one of each, and it
starts in `main`:

```rust,error
fn sneaky() -> Result<(), std::io::Error> {
    let mut out = std::io::Stdout {};
    out.line(&1i64)
}

fn main() {
    let _ = sneaky();
}
// error: `Stdout` is intrinsic: only its own functions make one or take one apart
```

What a function can do to the world is what it's handed: a stream, or a
`&mut` to one, as a parameter, in a struct, or captured by a closure.
That's *capability* passing (Dennis and Van Horn, 1966; the E language;
Pony's `env.out`; WASI), and a stream is to I/O what Clean's unique
`*World` is (Barendsen and Smetsers, 1993): one value, threaded through
the program, so what's done with it happens in an order the types see.

**Lemma C1 (no ambient authority).** Every I/O operation is an intrinsic
method taking `&mut` of a stream. A stream can't be written as a literal,
copied, or given back new by any intrinsic, and T has no global
variables. So every stream a run has descends, by moves, from the one
`Os` given to `main`, and a function reads or writes only through a
stream it was handed, or one in something it was handed.

**Lemma C2 (uniqueness).** At most one value of each stream type exists,
and borrows of it are those the borrow checker allows: so writes to it
happen in the order the program's data flow says.

The rule that would break C1 is a global: one holding a stream would be
authority anyone could reach. When T has globals (the reason `ALL_CAPS`
is reserved), they mustn't hold one.

## Not an effect

The other way to track I/O was an `io` effect, as Koka does: `print`
stays, and says `with io`. The proof would be the same as `panic`'s
(docs/07-effects.md). T chose values:

- **Which, not just whether.** An effect says a function does I/O, but not
  to what: one with `io` could write any file. A value says which stream,
  and files will be values too, linear ones closed by `close`, as the
  README's file descriptor that can't be forgotten.
- **Effects are for what isn't a value.** `panic` is control, `alloc` the
  program's own heap, `unsafe` an obligation. The world outside is a
  resource, and resources are values.

**Lemma C3 (effects unchanged).** Writing adds no effect: `Write for
Stdout` says `effect e = {}`, and `read_line` only `alloc`, for the line.
So a function with no `with` can still write, if it's handed a stream:
what "total" means is unchanged (no panic, no allocation, nothing
unsafe), and I/O is in its parameters instead.

```rust
fn count(out: &mut std::io::Stdout, n: u8) -> Result<(), std::io::Error> {
    for i in 0..n {
        out.put(&i)?;
    }
    out.line(&"")
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    count(&mut out, 4) // expect: 0123
}
```

## Errors are values

`Stdout` and `Stderr` are sinks, as a `String` is (docs/18-fmt.md), with
`type Error = std::io::Error`: the stream may be a pipe whose reader went
away, or a file on a full disk. So `out.put(&x)?` and `out.line(&x)?`
pass the error on. Both are methods of the streams themselves, too, so
writing doesn't need `Write` in scope.

`main` can give back `()`, or a `Result<(), E>`. An `Err` is written
where errors go, as `error: ` and then the error, and the program
fails, with status 1:

```rust
#[derive(Discard)]
struct Oops { n: i64 }

impl core::fmt::Display for Oops {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.put(&"oops ")?;
        w.put(&self.n)
    }
}

fn main() -> Result<(), Oops> { Result::Err(Oops { n: 2 }) }
// stderr: error: oops 2
```

**Lemma C4.** `E` must be `Display`, to be written, and `Discard`, as
it's thrown away after. Monomorphisation makes `std::rt::report` for
`main`'s `E`, which the interpreter calls with a `Stderr` of its own: if
writing that fails, there's nowhere left to say so.

A `Result<(), std::io::Error>` is `Discard`, as `Error` is, so nothing in
the types stops `out.line(&x);` from dropping an error. A warning does,
`unused_results`, for a statement whose value is a `Result` whose error
type isn't `!`; its fix, `let _ =`, says it's on purpose:

```rust
fn main(os: std::Os) {
    let mut out = os.stdout;
    out.line(&1i64); // expect: 1
}
// warning: this `Result` is thrown away, and any error in it unseen
```

## `?` meets linear values

`?` returns early, and a value that isn't `Discard` can't be dropped on
the way out (docs/15-try.md). In this step a `String` and a `List`
weren't, so a function holding one couldn't use `?` while it did. There
were three ways around it: work out what's to be written and free the
value first; write, free, and only then pass on what writing gave
(`let written = out.line(&s.as_str()); s.free(); written?;`); or the
*bracket*, a function that owns the value and frees it, calling one that
borrows it and does the `?`s. This isn't about I/O: every `Result` meets
it, and moving the tests and docs to `Stdout` met it about seventy-five
times.

Step 20 ended it for memory: a string and a list are `Discard` now, by a
`discard` of their own, which frees them on every path out, the early
ones of `?` too (docs/20-destructors.md):

```rust
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::from("café");
    out.line(&s.as_str())?; // expect: café
    out.line(&s.len())      // expect: 5
}
```

A value that's linear on purpose, as a file whose `close` can fail will
be, still can't be dropped on the way out: for one of those, the bracket
is still the way.

## Reading input

`stdin.read_line(&mut line)` puts the next line in `line`, in place of
what was there, without its line break, `\n` or `\r\n`, and gives
`Ok(true)`; `Ok(false)` if there's no more input. The last line needn't
end in a line break. Input that isn't UTF-8 is an error of kind
`InvalidData`. One `String` is used for every line, and freed once, when
it's dropped.

## What the checker trusts

Each stream has one intrinsic method, private to `std::io`: `read_byte`
or `write_byte`, giving a number, which `std` turns into a `Result`.
Everything else, UTF-8 encoding for `write_char`, the lines of
`read_line` and their check that they're UTF-8, is T, checked like any
code. `t run` flushes what's been written before each read and before
anything goes to stderr, so a prompt is seen before it's answered.

## Design decisions worth noticing

- **Capabilities, not an `io` effect**, for the reasons above. The cost:
  a closure with no effects can write through a `&mut Stdout` it
  captured, which is in its type only as what it borrows.
- **Errors as values, not a panic.** Rust's `println!` panics when the
  output's gone (`cargo ... | head`), and C's `printf` is usually
  unchecked. Here a write gives the error, and `?` passes it up to
  `main`, which reports it.
- **One idiom per test.** Tests and docs write with `out.line(&x)?`, and
  where a linear value is in the way, collect output in a `String`, whose
  `Error` is `!`, and write it at the end.
- **The `print` built-ins are gone**, and so is printing what `main`
  gives back, which was output nothing in the program's types said.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| files, arguments, the environment, clocks | each a field of `Os`, or a value one gives; files linear, with `close` | — |
| writing bytes, `std::io::Write` | text is all a program writes yet; bytes come with files | — |
| buffering in the program | `t` buffers; a compiled program will want a `BufWriter` written in T | — |
| a way for linear values to leave on `?` | destructors, `defer` or `try` blocks | 20 ✅ (destructors, for what's `Discard`) |
| an exit status other than 0 and 1 | an `ExitCode` for `main` to give back | — |

## Exercises

1. Write a `Shout` sink that wraps a `&mut Stdout` and writes everything
   in capitals. What are its `Error` and `e`?
2. Make `sneaky` above work by giving it a parameter. What does its
   caller have to have, and where did that come from?
3. Why must `Os` be passed by value, and not as `&Os` (`fn main(os:
   &Os)` is an error)? What would a borrowed `Os` allow?
4. In the first program, put the line in a `struct Line { s: String }` of
   your own, which isn't `Discard`. Where does the checker stop you, and
   why is it right? Then make it work with a bracket.
5. A global `static OUT: Stdout` would break lemma C1. Would a global
   holding a `fn(&mut Stdout)` break it too?

## Reading

- Jack Dennis and Earl Van Horn, *Programming Semantics for
  Multiprogrammed Computations* (1966), where capabilities come from;
  Mark Miller, *Robust Composition* (2006), on object capabilities and
  the E language.
- Erik Barendsen and Sjaak Smetsers, *Conventional and Uniqueness Typing
  in Graph Rewrite Systems* (1993): Clean's unique `*World`.
- Philip Wadler, *Linear types can change the world!* (1990).
- Austral's capability-based design (Fernando Borretti, 2023), a linear
  language that passes a root capability to `main`.
- Daan Leijen, *Koka: Programming with Row-polymorphic Effect Types*
  (2014), for the other way: `console` and `io` as effects.
