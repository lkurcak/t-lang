# Step 18: `core::fmt`

Code: `core/fmt.t` (`Write`, `Display`, and the impls for numbers, `bool`,
`char` and strings), `alloc/string.t` (`String` as a sink, `String::of`)
and `alloc/boxed.t`.

Until now a program couldn't turn a number into text. tcalc, the first
program written in T outside this repository
(<https://github.com/lkurcak/tcalc>), wrote its own `push_int`, and every
program printing a number among words would have to. Now a value is
written into a *sink*:

```rust
use alloc::string::String;
use core::fmt::Write;

fn main() with alloc {
    let mut s = String::new();
    s.write_str("x = ");
    s.put(&-42i64);
    s.write_char('!');
    print_str(s.as_str()); // expect: x = -42!
    s.free();
}
```

## Sinks and what can be written

A sink is where text goes. It says how writing to it can fail, by its
`Error`, and what writing to it does, by its effect `e`:

```text
pub trait Write {
    type Error;
    effect e;
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error> with Self::e;
    fn write_char(&mut self, c: char) -> Result<(), Self::Error> with Self::e;
    fn put<D: Display>(&mut self, x: &D) -> Result<(), Self::Error> with Self::e { x.fmt(self) }
}

pub trait Display {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e;
}
```

A `String` can't fail to take text, so its `Error` is `!`, and its `e` is
`alloc`, as growing may run out of memory. A `Result<(), !>` can only be
an `Ok`, and is `Discard`, so what writing to a string gives can be
dropped, as above.

Core has `Display` for the ten integer types, in decimal, for `bool`,
`char` and `str`, and for references to what has it; alloc for `String`
and `Box`. A program's own type says how it's written, from what's in it,
with `?` passing a sink's error on:

```rust
use alloc::string::String;
use core::fmt::{Display, Write};

#[derive(Discard)]
struct Point { x: i64, y: i64 }

impl Display for Point {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_char('(')?;
        w.put(&self.x)?;
        w.write_str(", ")?;
        w.put(&self.y)?;
        w.write_char(')')
    }
}

fn main() with alloc {
    let s = String::of(&Point { x: 3, y: -4 });
    print_str(s.as_str()); // expect: (3, -4)
    s.free();
}
```

`write_char` is a method of its own, not `write_str` of the character's
UTF-8, because there's nowhere to put the bytes: T has no arrays yet, so
no buffer on the stack.

## `fmt` does only what the sink does

`Display::fmt` says `with W::e`, and an impl's method can say no more
than its trait's (docs/07-effects.md). So the rule, applied to `fmt`:

- **F1 (effects).** Formatting a value into a sink `W` has at most the
  effects `W::e`. Formatting into a sink that has none is total: it can't
  panic, allocate, or do anything unsafe.

```rust
use core::fmt::Write;
use core::num::wrapping_add;

/// A sink that only counts the bytes written to it.
#[derive(Discard)]
struct Count { n: usize }

impl Write for Count {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        self.n = wrapping_add(self.n, s.len());
        Result::Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), !> {
        self.n = wrapping_add(self.n, c.len_utf8());
        Result::Ok(())
    }
}

/// How many bytes `x` takes in decimal. It says no effects at all.
fn width(x: i64) -> usize {
    let mut c = Count { n: 0 };
    c.put(&x);
    c.n
}

fn main() {
    print(width(-9223372036854775808)); // expect: 20
}
```

The cost is that a `Display` can't do anything a sink doesn't: one that
might panic doesn't fit the trait.

```rust,error
use core::fmt::{Display, Write};

#[derive(Discard)]
struct Next { n: i64 }

impl Display for Next {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.put(&(self.n + 1)) }
}
// error: `+` may panic, but `fmt` doesn't say `with panic`
```

That's a restriction that can be relaxed: if a real need shows up,
`Display` can get an effect of its own, as `Iterator` has, and every impl
that checks now still checks.

## Numbers without `panic`

F1 means core's own `fmt` of an integer can't panic, and nothing is
trusted about it: it's safe code, and the checker proves each operator in
it can't go wrong. It's written once, for every `T: Int`, and works on the
number as it is, negative or not, since the least value of a signed type
has no negation:

```text
fn digits<T: Int, W: Write>(m: T, w: &mut W) -> Result<(), W::Error> with W::e {
    let q = m / 10;
    if q != 0 {
        digits(q, w)?;
    }
    let r = m % 10;
    let d = if r < 0 { -r } else { r };
    w.write_char((d as u32 + 0x30) as char)
}
```

- **F2 (decimal).** Each operation in `digits` is defined, at every
  integer type. `m / 10` and `m % 10` divide by a constant that isn't 0
  or -1. The logic knows `r`, the remainder, has `m`'s sign and is less
  than 10 from 0 (docs/08-refinement-types.md), so `-r`, where `r < 0`,
  is at most 9, which fits in every type, and `d` is 0 to 9: `d as u32`
  fits, and `0x30 + d` is a character, `0` to `9`.

There are ten impls, `impl Display for i8` and so on, each a line calling
`decimal`, because one impl for every `T: Int` would overlap those for
`bool` and `char`: the overlap check looks at impls' headers, not their
bounds, so it doesn't know `bool` isn't an integer type, though `Int` is
sealed (that's sealed traits' disjointness, not in T yet).

## Errors are the sink's

What can go wrong is the sink's business, so each sink names its own
`Error`, and `?` passes it on, as it is. A sink whose error isn't
`Discard` can't have it dropped:

```rust,error
use core::fmt::Write;

/// Can't be dropped: it must be dealt with.
struct Lost {}

#[derive(Discard)]
struct Never {}

impl Write for Never {
    type Error = Lost;
    effect e = {};

    fn write_str(&mut self, _s: &str) -> Result<(), Lost> { Result::Err(Lost {}) }

    fn write_char(&mut self, _c: char) -> Result<(), Lost> { Result::Err(Lost {}) }
}

fn main() {
    let mut n = Never {};
    n.put(&1u8);
}
// error: this value is never used up, and its type, `Result<(), Lost>`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Lost`
```

- **F3 (linearity).** `fmt` takes `&self`, so formatting moves nothing,
  and a sink's error is a value like any other: if it's linear, it's
  used up, by the caller of the call that gave it. `Result<(), !>` holds
  nothing, so dropping it loses nothing.
- **F4 (borrows).** `write_str` is lent its `&str` for the call only: the
  signature ties its lifetime to nothing, so a sink that keeps it is
  rejected, as `tests/t/err_soundness_core_fmt_sink_keeps_str.t` checks.
- **F5 (no new trust).** `String`'s `write_str` and `write_char` are
  `push_str` and `push`, which keep it UTF-8 (docs/13.1-string.md): this
  step adds no `unsafe` and no intrinsic.

## `str` and `&str`

A type parameter never stands for `str`, as it's only ever behind a
reference (docs/10-slices.md). So `put("text")` doesn't check: `put`'s
`D` would be `str`. `impl Display for &str` is there for that, so
`w.put(&"text")` works, as does `w.write_str("text")`. It doesn't overlap
`impl<T: Display> Display for &T`, because the overlap check knows `T`
can't be `str`.

## Design decisions worth noticing

- **A sink's own error type, not one for all.** Rust has a single
  `fmt::Error`, which says nothing of what went wrong and makes writing
  to a string look as if it could fail. Here a string's is `!`, and a
  file's will be what a file's failures are.
- **Static dispatch.** `fmt` is generic over its sink, so each
  `Display` is made again for each sink it's written to: no `dyn`, which
  T doesn't have, and calls a backend can inline.
- **`put`, sink first.** `w.put(&x)?` reads in the order the text goes,
  and keeps `fmt`'s argument the sink, so a `Display` passes it on.
- **Not in the prelude.** Nothing in it says I/O yet, and std's byte
  sinks will want the name `Write` too.
- **The `print` built-ins are unchanged.** What printing does (an effect,
  or a value standing for the terminal) is std I/O's question.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| a `format!`-like way to write text with values in it | T has no macros, and string templates would be syntax with typing rules of their own; `put` and `?` cover it | — |
| width, padding, hexadecimal | wrapper types (`Hex(n)`, `Pad(..)`) implementing `Display`, not flags in a `Formatter` | — |
| `Debug` and `#[derive(Debug)]` | a second trait, and a derive that writes code, with strings escaped | — |
| `Display` for `Option` and `Result` | as in Rust: how to show one depends on the program | — |
| printing to the terminal through a sink | std I/O | next |

## Exercises

1. Write `Display` for an enum of your own with a variant holding
   another enum. Which `?`s can you leave out, and why only those?
2. Give `Display` an effect of its own, `effect e;`, and `fmt` `with
   {W::e, Self::e}`. What does `width` above then have to say? What would
   a generic `fn show<D: Display>(x: &D)` have to say?
3. In `digits`, replace `if r < 0 { -r } else { r }` with `-r` when `m <
   0`, tested before the recursion. Why doesn't it check? (The checker
   says, of a `T`, that a literal must be between 0 and 127.)
4. Write `impl<T: Int> Display for T` in place of the ten impls, in
   `core/fmt.t`. What does the checker say, and what would it need to
   know to accept it?
5. A `Bounded` sink takes at most `n` bytes and fails with how many were
   wanted (`tests/t/core_fmt_fallible_sink.t`). Writing `1234` into one
   with room for 2 writes `12`, then fails. Is that what you'd want of a
   sink? What would writing only whole values take?

## Reading

- Rust's `core::fmt` (`Write`, `Display`, `Formatter`), which this follows
  less the formatter, and the `ufmt` crate, a smaller one for embedded
  code with `uWrite`'s associated `Error`, as here.
- Guy Steele and Jon White, *How to Print Floating-Point Numbers
  Accurately* (1990), for what's ahead when T has floats: printing
  integers is the easy case.
- Philip Wadler, *A prettier printer* (2003): `t fmt`'s layout engine
  (`src/doc.rs`), a different kind of formatting, of documents that
  choose where to break lines.
