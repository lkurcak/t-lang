# Step 76: widths, captured fields and `mut`

Code: `pad`, `Align`, `dec` and the counting sink `Count` in `core/fmt.t`;
a hole's spec read by `lexer::spec`, written out by `FmtSpec::suffix`,
and made into calls by `Rewrite::format` (`src/check/modules.rs`); what a
closure captures in `Checker::capture_places` and `Checker::cut`
(`src/check/closures.rs`), with `Typed::place_path`; where a closure's
body finds what it captured in `Builder::captured_place`
(`src/borrowck/build.rs`) and `Walk::captured_root`
(`src/check/refine/mir.rs`); and the `mut` shorthand in
`Parser::struct_pattern`.

t-life, the program on several threads (docs/41-threads.md), wrote three
things by hand that it shouldn't have had to:

- A table of timings, padded a character at a time: a hole of a format
  string could say `:?` or `:x`, but not how wide it is.
- `let clock = os.clock;` before a closure that named `os.clock`, because
  the closure took all of `os`, which a `let` had already moved a field
  out of.
- `let Band { rows, next: mut next } = band`, where Rust says
  `Band { rows, mut next }`.

Now:

```rust
#[derive(Discard)]
struct Run { name: alloc::string::String, ms: u64, clock: u64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let run = Run { name: alloc::string::String::from("pool"), ms: 1234, clock: 7 };
    let name = run.name;
    // Only `run.clock` is captured: `run.name` is gone, and that's fine.
    let tick = || run.clock;
    let Run { mut ms, .. } = run;
    ms = ms + tick();
    out.line(&f"[{name:<6}][{ms:>8}][{ms:05}]") // expect: [pool  ][    1241][01241]
}
```

## Widths

After a hole's `:` can come a width: a side, `<` for the start, `^` for
the middle or `>` for the end, then how many characters at least, from
1 to 65535. Before the side can come the fill, any character but a
brace, a quote or a backslash; a space if there's none. After the width
can come `?` or `x`, as without one:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (h, w, s) = (255u8, 6usize, "é█");
    out.line(&f"[{h:*>6}][{h:-^7x}][{s:<4?}][{s:.^w$}]") // expect: [***255][--ff---]["é█"][..é█..]
}
```

The width counts characters, as Rust's does, not bytes: `é█` is 2 of
them, in 5 bytes. That's what a terminal shows of most text, though not
of characters twice as wide, nor of combining ones. In the middle, the
odd fill goes after the value.

A width can be a variable's, `{x:>w$}`, a `usize`: the variable is
looked up where the string is, as a hole's value is, and is a use of it.

A width says which side: `{x:8}` is refused. Rust puts a number at the
end of its width and anything else at the start, but a format string is
made into calls before types are known, so the side would have to be a
method of `Display`; saying which is clearer, and the rule can be eased
later without changing what any program means.

```rust,error
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 10u8;
    out.line(&f"{n:8}") // error: say which side of the width `n` goes: `{n:>8}`, `{n:<8}` or `{n:^8}`
}
```

`{n:05}` is an integer in decimal in at least 5 characters, with 0s after
its sign: `-42` is `-0042`. It's to decimal what `{n:08x}` is to
hexadecimal, and like it doesn't combine with a side.

### What a width is made of

`{x:*>8}` is `pad(display(&x), 8, '*', Align::Right)`, with the core
library's `pad`; `{x:>8?}` is the same of `debug(&x)`. `pad` is a function
like any other, for a width or fill worked out as the program runs:

```rust
use core::fmt::{Align, display, pad};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 42u8;
    out.line(&pad(display(&n), 5, '.', Align::Right)) // expect: ...42
}
```

`Pad`'s `Display` writes its value twice: first into a sink that only
counts characters, up to the width, and is then full; then into the real
sink, after whatever fill goes first, with the rest of the fill after.
The counting sink's `Error` is `()`, so a value wider than the width
stops being counted as soon as that's known, and its effect is `{}`, so
the first pass can do nothing at all: writing a value does only what its
sink does (docs/18-fmt.md). It's plain T in `core`, with nothing unsafe,
and checked as any: the count can't overflow, as it stops at the width.

Writing a value twice gives the same text twice, but for a `Display`
that changes itself through a `Cell` as it's written, which is then
padded by what it wrote the first time. Nothing depends on that being
right: padding is what's written, not what's known.

## `mut` in a struct pattern

`S { a, mut b }` is `S { a, b: mut b }`, as `S { a }` is `S { a: a }`,
and `t fmt` writes it the short way. Renaming the variable spells it out,
`S { b: mut c }`, and the quick fix for an unused one is
`S { b: mut _b }`.

## Closures capture the places they name

A closure used to capture whole variables: `|| p.x` borrowed all of `p`
(docs/06c-closures.md). Now it captures the places its body names, as
Rust has since 2021: `|| p.x` borrows `p.x`, and the function can use
the rest of `p` while the closure lives:

```rust
#[derive(Discard)]
struct S { count: u64, step: u64 }

impl S {
    fn bump(&mut self) with panic {
        let count = &mut self.count;
        // Borrows `(*self).step`, not `self`.
        let step = || self.step;
        *count = *count + step();
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut s = S { count: 1, step: 3 };
    s.bump();
    let mut p = S { count: 0, step: 1 };
    let mut a = || p.count = p.count + 1;
    let mut b = || p.step = p.step * 2;
    a();
    b();
    out.line(&f"{s.count} {p.count} {p.step}") // expect: 4 1 2
}
```

How far a capture goes depends on how it's captured:

> **K1. By `&`, the place named.** Through fields, references and boxes:
> `|| r.x` for `r: &mut S` borrows `(*r).x`.
>
> **K2. By `&mut`, uniquely or by value, fields of the variable.** Not
> through a reference or a box: a closure that changes `r.x` captures `r`
> uniquely, and `move || r.x` takes `r`. Nor into a variable declared
> with a refinement, nor into a struct whose fields have refinements:
> those are captured whole.
>
> **K3. Moved, not out of what can't be taken apart here.** A capture by
> value of a place that isn't `Copy` moves it out: not out of a struct
> with a `discard` of its own, outside its module, nor of one with
> private fields that isn't `Discard` (lemma D4 of
> docs/20-destructors.md). Those are moved in whole.
>
> **K4. Apart.** Where a closure names a place and a place in it, it
> captures the outer one, the most either needs, and cuts that again by
> K2 and K3; so what it captures are apart, and each place it names is in
> one of them.

How it's captured is as before: by `&` if the body only reads it, by
`&mut` if it changes it, uniquely if it changes what a `&mut` there
points to, by value if it moves it out or the closure is `move`.

A variable that a predicate in the closure names, a loop's invariant or
a `let`'s refinement, is captured whole: the predicate is about the
variable.

What a closure owns is dropped with it, and what it didn't take stays
where it was: `move || take(p.name)` leaves `p`'s other fields to be
dropped where `p` is, not where the closure is. That's a change: before,
the closure owned all of `p`, and dropped it all.

```rust,error
#[derive(Discard)]
struct R { lo: u32, hi: { h: u32 | h >= lo } }

fn main() {
    let mut r = R { lo: 0, hi: 10 };
    // `R`'s fields have refinements: `r` is captured whole.
    let mut c = || r.lo = 5;
    r.hi = 3; // error: cannot assign to `r.hi` while `r` is mutably borrowed
    c();
}
```

## Why it's sound

> **Lemma C1 (the body sees the same places).** A closure's body sees
> each place it names through the closure's field for the capture it's
> in (`Builder::captured_place`), and the borrow checker and the
> refinement checker see that as the place it is in the variable, as
> they did when the whole variable was captured
> (`Walk::captured_root`). So everything checked of the body is checked
> of the same places as before.

> **Lemma C2 (making it is borrowing or moving the captures).** Making
> the closure is the MIR an explicit `&q`, `&mut q` or move of `q` would
> be, for each capture `q`. So the borrow checker's rules for those hold
> of it: loans of places apart don't clash (lemma K4 makes them apart),
> and moving `q` out of its variable is a move of part of it, whose rest
> is dropped, or must be used up, as after any such move.

> **Lemma C3 (nothing accepted is refused).** A capture is cut where it
> couldn't be made (K2: not out from behind a reference; K3: not out of
> what can't be taken apart), to what was captured before at that
> point; so the borrow checker accepts every capture it accepted when
> whole variables were captured.

> **Lemma C4 (refinements don't straddle a capture).** The refinement
> checker walks a closure's body from what was known where it was made
> (lemma S3 of docs/69-loans.md). Before, everything the body could read
> or had to keep was in a capture: frozen by a `&`, owned, or forgotten
> by a `&mut`'s loan. Now the rest of a variable, next to a capture, is
> the function's, and may change while the closure lives. The body
> never reads it: it'd be in a capture. But a change the body makes must
> keep the refinements about the place it changes, and one of those
> could also be about the rest: a struct's field refinements relating
> its fields, a variable's own refinement, or what's declared behind a
> reference. The walk would check them against the rest as it was when
> the closure was made. K2 cuts a changing capture at each of those, so
> none relates a place in it to one outside it; refinements of type
> arguments are of values, not places (lemma A1 of
> docs/60-refined-type-arguments.md). As a backstop, the refinement
> checker refuses, as an internal error, a closure that captures by
> `&mut` or by value a place a refinement relates to the rest
> (`Walk::captures_apart`).

> **Lemma C5 (`&` captures go anywhere).** A capture by `&` freezes its
> place while the closure lives, so what was known of it when the
> closure was made stays true; what was known relating it to the rest is
> about values the rest had, which the body can't name, and nothing it
> concludes is about them. A closure's summary (docs/74) is of values
> too. So K1 needs no cut, and a closure reading `self.step` can live
> next to a `&mut self.count`.

The adversarial tests are `tests/t/err_soundness_capture_*.t`,
`err_capture_*.t` and the generated family in
`tests/generated_captures.rs`: a closure reading, changing, copying or
moving a field of a variable, of what a `&mut` points to, or of what a
box holds, while the function uses the field next to it, the same one,
one further off, or the struct it's in. Each program that clashes is
refused; each that doesn't runs.

## What it doesn't do

- **`&mut` through references.** A closure that changes `self.b` in a
  `&mut self` method captures `self` uniquely, so `self.a` can't be
  borrowed next to it, as it can in Rust. Going through the reference
  would need the refinement checker to know nothing is declared behind
  it relating `b` to the rest, which it can't from the type alone.
- **Places in patterns.** `match p { P { x, .. } => .. }` in a closure
  uses all of `p`, as does a `let` taking it apart; Rust captures only
  what the pattern reads.
- **Elements.** `|| a[0]` uses `a`: an index is a call.
- **A side by type.** `{x:8}` is refused, as above.
- **Precision.** `{x:.2}` has nothing to mean: T has no floating-point
  numbers yet.
- **Widths in columns.** A character twice as wide as others, as some
  of Chinese, counts as one.

## Decisions

- **D1: a width says its side.** `{x:8}` is refused, not Rust's default
  by type.
- **D2: a width counts characters.** As Rust's, not bytes.
- **D3: `{n:05}`, `{n:>w$}` and a fill.** All three, as Rust has them.
- **D4: K1 to K4.** `&` captures through references, `&mut` and value
  captures fields only, cut at refinements; not Rust's whole rule, which
  also lets a `&mut` capture go through a reference.

## Exercises

1. `pad` writes its value twice. Write a `Pad` that writes it once, with
   a buffer: what does it need that `core` doesn't have?
2. Why does the counting sink fail when it's full, rather than count on?
3. Write the program K2's refinement cut is for: a closure changing one
   field, the function another, and a struct refinement between them
   broken. What would the refinement checker say if the closure captured
   the field alone?
4. K3 cuts a `move` closure's capture of `f.name` at `f` when `File` has
   a `discard` of its own. What would go wrong if it didn't?
5. Rust captures only what a pattern in a closure reads. What would T
   need, here, to do so?

## Reading

- RFC 2229, *Closures capture disjoint fields*: the rules this follows,
  and the drop order they change.
- The Rust reference, *Closure expressions: capture modes* and
  *Format strings: width, fill and alignment* (`std::fmt`).
- Niko Matsakis, *Closures capture disjoint fields*, the design notes
  for RFC 2229, on truncating at derefs.
