# Step 56: field paths and proved sub-slices

Code: `Checker::pred_place`, `field_reach`, `pred_mutable` and
`Walk::term` in `src/check/refine.rs`, with `Walk::same_measures` and
`Walk::post`; `Refined::eval` and `path_root` in `src/interp.rs`; `cut`,
`split_at`, `slice` in `core/slice.t`, the lengths in `core/str.t`;
`Checker::implicit_panics` (`Lint::ImplicitPanics`); `group_of` in
`src/check/modules.rs`; `Checker::only_impl` in
`src/check/traits/methods.rs`.

A small batch, from retrospective 5 and step 55. A refinement could name
a variable, and call a measure of one, but not read a field. So
`split_at` couldn't say how long its halves are: they're the fields
`head` and `tail` of what it gives. And with no way to say that, a slice
of a slice couldn't be proved in range, which is what the grid in
t-libs needed for its rows.

Now a predicate names fields, and core says what it knows:

```rust
// A row of a grid stored row by row.
fn row(
    cells: &[u8],
    w: usize,
    y: { y: usize | y * w + w <= cells.len() },
) -> { r: &[u8] | r.len() == w } { cells.slice(y * w..y * w + w) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let cells: [u8; 6] = [1, 2, 3, 4, 5, 6];
    let r = row(cells.as_slice(), 3, 1);
    out.line(&r[2])?; // expect: 6
    match cells.as_slice().split_at(4) {
        Some(s) => out.line(&s.1[1])?, // expect: 6
        None => {},
    }
    Ok(())
}
```

`r[2]` needs `2 < r.len()`, which `row`'s result says; `s.1[1]` needs
`1 < s.1.len()`, which `split_at` says now: `s.1.len() + 4` is the
slice's length, 6.

## Fields in predicates

A predicate can name a field of a struct it can name: a parameter's, a
`let`'s, the binder's, a field before it in a struct, at any depth, and
through `&`, `&mut` and boxes, as `p.x` reads one in code. Then an
integer or a `bool` field is a value, and a field that's a struct or a
reference has fields and measures of its own: `l.from.x`,
`s.0.len()`.

```rust
#[derive(Copy, Discard)]
struct Point { x: u8, y: u8 }

#[derive(Discard)]
struct Counter { n: u8 }

fn x_of(p: &Point) -> { v: u8 | v == p.x } { p.x }

// A field of a field before it, in a struct's refinement.
#[derive(Copy, Discard)]
struct Span { from: Point, len: { n: u8 | n <= 255 - from.x } }

fn end(s: Span) -> u8 { s.from.x + s.len }

// What's behind a call-scoped `&mut` at return, and with `old(..)`, at
// the call.
fn reset(c: &mut Counter) -> { v: u8 | v == old(c.n) } ensures c.n == 0 {
    let v = c.n;
    c.n = 0;
    v
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Point { x: 2, y: 5 };
    let a: { a: u8 | a == 2 } = x_of(&p);
    out.line(&a)?;                                                  // expect: 2
    out.line(&end(Span { from: Point { x: 250, y: 0 }, len: 5 }))?; // expect: 255
    let mut c = Counter { n: 7 };
    let was: { w: u8 | w == 7 } = reset(&mut c);
    let now: { z: u8 | z == 0 } = c.n;
    out.line(&(was + now))?; // expect: 7
    Ok(())
}
```

A field is a function of the value it's in, as a measure is: a struct's
value, to the logic, is its fields' values. So nothing new is needed to
say what one is; what's needed is that the field is there to be read and
won't change while the predicate is about it.

**What can change.** A field behind a `&mut` follows the rules a
measure of what's behind one does (docs/21-measures.md): the refinement
of a `mut` variable or parameter, or of a struct's field, is checked
again later, so it can't name one; a postcondition can, of a call-scoped
`&mut` parameter, whose value at return is what the caller reads next,
or inside `old(..)`.

```rust,error
#[derive(Discard)]
struct Counter { n: u8 }

fn f(c: &mut Counter) {
    let mut i: { i: u8 | i <= c.n } = 0;
    i = 0;
    let _ = i;
}
// error: the refinement of a `mut` variable can't name a field of what's behind `&mut` `c`, which can change
```

**Who can see it.** A predicate names only fields it can see, as code
reads only those. And a signature's refinement names only fields seen at
least as far as the function is: a `pub` function's at least `pub`, an
exported one's exported. A function's callers prove its preconditions
and use its postconditions; were those about a field they can't see,
their proofs would hang on what its module keeps to itself, and break
when it changes. A measure is what a module offers instead: outside its
module, one that reads fields is only what's been learned of it.

```rust,error
mod shapes {
    #[derive(Copy, Discard)]
    pub struct P { x: u8 }

    pub fn x_of(p: P) -> { v: u8 | v == p.x } { p.x }
}
// error: `x_of` is public, but its refinement names field `x` of `P`, which is private to `shapes`
```

A struct's field's refinement is the same, as far as the field is seen.
An impl's method of a trait is seen wherever the impl is, which is
wherever its trait and type are named: so what its result's refinement
names must be exported.

## How long things are

What core gives now says how long it is:

- `s.split_at(mid)` and `split_at_mut`, of a slice or a `str`:
  `Option<{ s: (&[T], &[T]) | s.0.len() == mid && s.1.len() + mid ==
  self.len() }>` (a `Split { head, tail }` until step 58's tuples);
- `s.get(lo, hi)` of a `str`: `Option<{ t: &str | t.len() + lo == hi
  }>`;
- `trim`, `trim_start`, `trim_end`, `strip_prefix`, `strip_suffix` of a
  `str`: no longer than it;
- a string literal: as long as its text.

`find` doesn't: it's a method of a `P: Pattern`, a trait's, which can't
have refinements yet.

`split_at` used to be trusted, an intrinsic that gave `None` past the
end. It's safe T now, over a private intrinsic, `cut`, that takes an
index proved in range and gives the two halves, with their lengths:

```text
#[intrinsic]
fn cut<'a>(&'a self, mid: { m: usize | m <= self.len() }) -> {
    s: (&'a [T], &'a [T]) | s.0.len() == mid && s.1.len() + mid == self.len()
};

export fn split_at<'a>(&'a self, mid: usize) -> Option<
    { s: (&'a [T], &'a [T]) | s.0.len() == mid && s.1.len() + mid == self.len() },
> {
    if mid <= self.len() { Some(self.cut(mid)) } else { None }
}
```

The trusted code is no bigger: one intrinsic was replaced by another,
which checks, as `index` does, what its refinement already says. And now
code that has the proof needs no `None`.

## Sub-slices

`s.slice(r)` is the values of `s` from `r.start()` up to, not including,
`r.end()`, for a range proved in it:

```text
export fn slice<'a>(
    &'a self,
    r: { r: Range<usize> | r.start() <= r.end() && r.end() <= self.len() },
) -> { t: &'a [T] | t.len() + r.start() == r.end() } {
    let head: &'a [T] = self.cut(r.end()).0;
    head.cut(r.start()).1
}
```

`a..b` is `Range::new(a, b)`, whose result says its `start()` is `a` and
its `end()` is `b` (docs/53-refined-type-arguments.md), so a call's
obligation is `a <= b && b <= s.len()`. `slice_mut` is the same, to
change; arrays and lists forward both to their slices.

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: [u8; 4] = [1, 2, 3, 4];
    a.slice_mut(1..3)[1] = 9;
    out.line(&a[2])?; // expect: 9
    let mut l: List<u8> = List::new();
    l.push(5);
    l.push(6);
    let t = l.slice(1..2);
    out.line(&t[0])?; // expect: 6
    Ok(())
}
```

```rust,error
fn third(s: &[u8]) -> u8 {
    if s.len() < 4 {
        return 0;
    }
    s.slice(1..5)[0]
}
// error: can't prove `r.start() <= r.end() && r.end() <= self.len()` for argument `r` of `slice`
```

## Implicit panics

An operator the checker can't prove won't fail keeps its check, and
`panic`s if it does: `i - 10` of an `i` that may be less than 10. Effect
inference (docs/24-visibility.md) gives a function that isn't exported
the effects of its body without its saying so, so nothing in the program
shows where that can happen. The lint `implicit_panics`, off unless
asked for, warns of each such operator, with what its proof was missing.
With `#![warn(implicit_panics)]` at the top and `t check
--deny-warnings`, a program is checked to have none:

```rust
#![warn(implicit_panics)]

fn helper(i: usize) -> usize { i - 10 }

fn safe(i: { i: usize | i >= 10 }) -> usize { i - 10 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&helper(20))?; // expect: 10
    out.line(&safe(20))?;   // expect: 10
    Ok(())
}
// warning: `-` may panic: it wasn't proved not to
```

Its notes are those of a failed proof (docs/08-refinement-types.md): the
condition it needed and what was known. Operators in closures count,
and so do those of a function that says `with panic`: saying it may
panic doesn't say where.

## `use Enum::{self, A}`

`self` in a `use` group names what the group is in: a module, and now an
enum too, whose variants the rest of the group are.

```rust
mod shapes {
    #[derive(Discard)]
    pub enum Shape { Dot, Line(u8) }
}

use shapes::Shape::{self, Dot, Line};

fn length(s: Shape) -> u8 {
    match s {
        Dot => 0,
        Line(n) => n,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&length(Line(3)))?; // expect: 3
    out.line(&length(Dot))?;     // expect: 0
    Ok(())
}
```

## The only impl

`List::new()` gives a `List<T>` for a `T` nothing has said yet. Writing
to it, `l.write_all(b"hi")`, found `io::Write`'s method, but the type
stayed unknown: the impl that would have said it, `impl Write for
List<u8>`, wasn't used to. Now a method of a trait, called on a value of
a struct or an enum whose type isn't known whole, is of the one impl of
that trait whose type it could be, if there's one, and that impl's type
says the rest:

```rust
use alloc::list::List;
use core::io::Write;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l = List::new();
    l.write_all(b"hi")?;
    out.line(&l.len())?; // expect: 2
    Ok(())
}
```

Not if a bound in scope could be how it implements the trait, nor of the
language's own types and closures, whose impls are partly built in. And
it's only a method call's: a `List::new()` passed to a function that
takes any `W: Write` still needs its type written.

## Why it's sound

- **Lemma F1 (a field is a function of the value).** The logic's value
  of a struct is its fields' values (`Val::Struct`), read from the place
  it's at, or made by a literal or a call, with the struct's refinements;
  a field of it is the one at its index. Through a reference, it's what
  the reference points to, read when needed; through one that isn't
  known, nothing (`Failure::Unsupported`: no fact, and no proof).
- **Lemma F2 (it doesn't change).** A field named in a predicate is of a
  value that can't change while the predicate is about it: through `&`
  nothing in T changes what it points to (a cell's fields are private);
  through a `&mut`, only where the measure rules allow (docs/21, 30):
  once, at a call or at return of a call-scoped parameter, or in
  `old(..)`. Each value is a variable of the logic, a new one whenever its
  place may have changed, so a fact stays true of the value it was
  about.
- **Lemma F3 (abstraction).** A field named in a signature's refinement
  is seen wherever the function is, so whatever proves or uses that
  refinement can see it: a module's private fields stay its own
  business, as lemma M2 of docs/06d-modules.md has it for code.
- **Lemma S1 (sub-slices).** `cut` checks `mid <= len` and gives the two
  halves, as `index` checks `i < len`; its refinement is what it does, as
  the interpreter checks at every call. `split_at`, `slice` and the rest
  are safe T over it, each proved by the checker.
- **Lemma S2 (equal values, equal measures).** Where branches meet, or a
  free theorem says a result is one of its arguments, a value equal to
  another has the same measures: a measure is a function of the value,
  and a slice's of the reference. So the joined slice's length is
  whichever branch's it was, each under the condition that it's that
  branch's (`Walk::same_measures`).
- **Lemma S3 (a lent reference is the same reference).** After a call
  that's lent a call-scoped `&mut`, what's behind it is what the call left
  there, but the reference is the one the caller had: a slice's length,
  of the reference, is the same (`Walk::post`). An intrinsic's `&mut`s
  count as call-scoped, so `cut_mut`'s lengths are learned.
- **Lemma I1 (the only impl).** Every impl of a trait for a struct or an
  enum is in the package of the trait or of the type (lemma O of
  docs/29-closed-and-sealed-traits.md), so all of them are seen wherever
  both are named, and adding a dependency can't add one. If only one
  could be for the receiver's type, any other type it could turn out to
  be implements nothing: the program would be an error anyway. The
  obligation that it implements the trait stays, and that impl meets it.
- **The interpreter checks.** `t run --interp` evaluates each predicate
  with fields, reading them from the struct's value; a postcondition's
  fields and measures that aren't behind a `&mut` are found at the call,
  when what they're of is still there (an owned box is dropped by
  return).

## What changed underneath

- `Warning` has notes, which `implicit_panics` uses for its proof's.
- Measures of equal values are equated where branches meet and where a
  free theorem gives one of several arguments: step 55's "what's known
  of them besides their integers and enums' parts (a measure) isn't
  carried over" is no longer so.
- A const named as if it had a field or a measure, `C.len()`, crashed
  the checker since step 21; it's an error now.

## What it doesn't do

- A field of an enum's variant: an enum has no fields to name. A
  refinement of its type argument says what's in a variant
  (docs/53-refined-type-arguments.md).
- What `Box::new(v)` gives isn't known to point to `v`: a predicate of
  `b.y` holds in a function given a box, but its caller learns nothing
  of the box it made.
- `find` and `Pattern`'s methods: a trait's method can't be refined.
- `slice` of a `str`: where characters start isn't in the logic.
- The only impl decides a method call's receiver, not an obligation from
  a generic function's bound.
- After `l.slice_mut(..)`, a list's length is forgotten: the borrow it
  gives back outlives the call, so it can't promise what it leaves.

## Decisions

- D1, field paths: fields visible at the predicate and at least as far
  as the item, rather than measures on `Split` only core could see
  through, or no fields.
- D2, sub-slices as `s.slice(a..b)` with a range refined by its measures,
  rather than `s.sub(a, b)` or only `get(a..b) -> Option`.
- D3, `implicit_panics` off by default, every unproved operator,
  `with panic` functions' too, rather than only those whose effects were
  inferred, or on by default.
- D4, a method call decided by the only impl that could be its, rather
  than always asking for an annotation.
- (Along the way) `split_at` as safe T over a private total intrinsic,
  so the trusted code stays the same size and `slice` needs no `None`.

## Exercises

1. Write `fn swap_halves(s: &mut [u8], mid: { m: usize | m <= s.len() })`
   with `split_at_mut`. Which lengths does the checker know, and from
   what?
2. Why may a postcondition name `c.n` of a call-scoped `c: &mut Counter`,
   but a `mut` variable's refinement may not? What would go wrong?
3. A library exports `fn area(r: Rect) -> { a: u32 | a == r.w * r.h }`
   with `w` and `h` private. Why is that rejected? Rewrite it with
   measures.
4. Find a program the only-impl rule accepts whose meaning would change
   if `alloc` added `impl Write for List<u16>`. Why can't a dependency
   of yours add it?

## Reading

- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008): refinements over
  measures and fields of data.
- Vazou, Seidel, Jhala, Vytiniotis and Peyton Jones, *Refinement Types
  for Haskell* (2014): measures, and what's sound about them.
- Jones, *Simplifying and Improving Qualified Types* (1995): improvement,
  using what instances exist to decide a type.
- The Rust reference, *Method call expressions*, and rustc's handling of
  a sole applicable impl in type inference.
