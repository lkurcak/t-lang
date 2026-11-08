# Step 53: refinements in type arguments

Code: `Checker::refined_arg`, `Walk::enum_parts`, `Walk::part`,
`Walk::callee` in `src/check/refine.rs`; `Range` in `core/ops.t`.

A refinement used to be only the whole type of a parameter, a result, a
`let` or a struct's field (steps 8, 8.1). So a function that may not have
an answer couldn't say anything of the answer it has:

```rust
fn digit(c: u8) -> Option<{ d: u8 | d < 10 }> {
    if c >= 48 && c <= 57 { Some(c - 48) } else { None }
}

fn value(c: u8) -> u8 {
    let squares: [u8; 10] = [0, 1, 4, 9, 16, 25, 36, 49, 64, 81];
    match digit(c) {
        Some(d) => squares[d as usize], // `d < 10`, so this can't panic
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&value(55))?; // expect: 49
    out.line(&value(65))?; // expect: 0
    Ok(())
}
```

And `for i in 0..s.len()` didn't know `i < s.len()`, as `Range`'s `next`
gave an `Option<T>`. Now it does:

```rust
fn sum(s: &[u8]) -> u64 {
    let mut total: u64 = 0;
    for i in 0..s.len() {
        total = total + s[i] as u64;
    }
    total
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [1, 2, 3, 4];
    out.line(&sum(a.as_slice()))?; // expect: 10
    Ok(())
}
```

## Refinements of an enum's type arguments

Wherever a refinement can be, a type argument of an enum there can be one
too, at any depth: `Option<{ d: u8 | d < 10 }>`, `Result<{ v: u8 | v < 10
}, { v: u8 | v >= 10 }>`, `Option<Option<{ i: usize | i < 4 }>>`, or one
of the program's own enums. It's about the fields of the variants whose
type is that parameter, `Some`'s for `Option`: each has it, if the value is
that variant. A `None` has nothing it could be about, so it has every such
type.

Only an enum's, and only a parameter its variants have as whole fields. A
`Box<T>`'s `T` is behind a pointer, a struct's fields may be private, and a
`List<T>`'s are in memory the logic doesn't see: there'd be no way to check
it of every way such a value is made. Nor a `T` inside a field:

```rust,error
use alloc::boxed::Box;

enum Tree<T> { Leaf, Node(Box<Tree<T>>, Box<T>) }

fn f(t: Tree<{ v: u8 | v < 3 }>) {}
// error: `T` is in a field of variant `Node` of `Tree` other than as its whole type
```

What's said of the payload holds only if the value is that variant. So a
refinement that can't hold, as `v < n` with `n` 0, says nothing of a
`None`, and the arm that matched it learns nothing it could prove anything
with:

```rust,error
fn below(n: u8) -> Option<{ v: u8 | v < n }> {
    if n > 0 { Some(n - 1) } else { None }
}

fn f() -> u8 {
    let a: [u8; 1] = [7];
    match below(0) {
        Some(_) => 0,
        None => a[5],
    }
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

A refined `let mut` of such a type is an invariant, as any refined `let
mut` is (step 8): each assignment must keep it, and it can't be borrowed
`&mut`, which a `match` on `&mut o` would do. A struct's field with one
can't be lent `&mut` either (step 8.1).

## Enums in the logic

To check such a refinement, the checker has to know what an enum's value
is. It knew nothing of one before: `let o = Some(3)` was a value of type
`Option<u8>`, nothing more. Now an enum's value has *parts*: which variant
it is, a number, and the values of each variant's fields. A literal's are
known; a pattern says which variant it is, in its arm, and what its fields
are; and where a value of a refined type is bound, each refinement of a
type argument is known of it, under the condition that it's that variant:
`tag == Some ⇒ d < 10`. What a branch gives is whichever branch's parts,
as a struct's fields are. So this is proved too, with no refinement at
all:

```rust
fn third() -> u8 {
    let o = Some(3u8);
    let t: [u8; 5] = [1, 2, 3, 4, 5];
    match o {
        Some(i) => t[i as usize],
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&third())?; // expect: 4
    Ok(())
}
```

## An impl can say more than its trait

`Range`'s `next` is `Iterator`'s, and a trait's methods can't have
refinements: every impl would have to keep them, and `Iterator`'s `next`
can't say anything of what every iterator gives. But a call of `next` on a
`Range<usize>` is a call of `Range`'s impl, and the checker knows that: so
an impl's method can now refine its result, and ensure more, and a call
the checker knows is of that impl learns it.

```rust
#[derive(Discard)]
struct Countdown { left: u8 }

impl Iterator for Countdown {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u8 | v < 10 }> {
        if self.left > 0 && self.left <= 10 {
            self.left = self.left - 1;
            Some(self.left)
        } else {
            None
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let digits: [u8; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    let mut sum: u16 = 0;
    let c = Countdown { left: 4 };
    for d in c {
        sum = sum + digits[d as usize] as u16;
    }
    out.line(&sum)?; // expect: 30
    Ok(())
}
```

A call is known to be of an impl when the type its `Self` is has nothing in
it that could be any type: no type parameter, associated type or `impl`
type. Through a type parameter, `next` is the trait's, and nothing is known
of what it gives:

```rust,error
fn first<I: Iterator<Item = u8>>(it: &mut I) -> u8 {
    let digits: [u8; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    match it.next() {
        Some(d) => digits[d as usize],
        None => 0,
    }
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

An impl's method still can't refine its parameters: a call through the
trait couldn't know to check them.

## `for` over a range

`Range` is now of integers only, `T: Int`, which is all it was used for,
and says what it is: `start` and `end` are measures, and

```text
fn new(start: T, end: T) -> { r: Range<T> | r.start() == start && r.end() == end }
fn next(&mut self) -> Option<{ v: T | v < self.end() }> ensures self.end() == old(self.end())
```

`for x in 0..n` lends its range to `next` each time round, which keeps the
range's end, so the loop's frame keeps it (step 52), and each `x` is less
than it: `n`. A measure can now give an integer of a type parameter `T:
Int`, which, where it's called, is an integer every integer type has room
for.

A frame used to come only from a method call, `l.set(i, v)`, and not from
a call by path, `List::set(&mut l, i, v)`, which is what `for` makes of
`next`. Now both lend `l` straight to the call.

## A hole this step closed

A call by path lent its `&mut` argument without saying which place it was,
so what the callee ensured of the value after the call was learned of the
value before it: `List::push(&mut l, x)` said `l.len() == l.len() + 1`,
from which anything follows. Every release since 0.43.0 (step 30) accepted
this:

```rust,error
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::new();
    List::push(&mut l, 0u8);
    let a: [u8; 1] = [7];
    out.line(&a[5])?;
    Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

So did any call given a `&mut` that isn't a place, as `f({ &mut l })`.
Now a call by path lends as one by name does, and a call-scoped `&mut`
argument whose place isn't known is a new value after the call, of which
nothing's known.

## Why it's sound

- **Lemma E1 (parts).** An enum's value is one variant, with that
  variant's fields. The checker's tag and fields of an object are
  variables of the logic, the same each time the object is asked, as a
  measure's are (lemma F1 of docs/52-loop-frames.md); an object is one
  value, and a place that may have changed is a new object. Facts about
  them come only from a literal, which is that variant with those fields;
  from a pattern, true in the arm it matched; from a branch, of the value
  the branch taken gave; and from a refinement, below.
- **Lemma E2 (refined type arguments).** Every value of an enum type with
  a refined type argument has it: of each field the refinement is about,
  if the value is that variant. A value gets such a type where a
  refinement is checked: a literal, whose fields the checker knows; a
  value that has a stronger refinement, through facts; or nothing else,
  as what generic code gives, `unwrap` or `map`, is a value nothing's
  known of, which can't be proved to have one. A field it's about is a
  whole field, so a pattern reads it, and a `let mut`, a parameter or a
  struct's field of such a type can't be written through a `&mut` (steps 8
  and 8.1), so every way to change one is an assignment of a whole value,
  which is checked. A `None`, or any variant the refinement isn't about,
  has it vacuously, and the facts say so only under the condition that the
  value is that variant: so a refinement that can't hold makes no
  contradiction.
- **Lemma R1 (impls).** At a call of a trait's method whose `Self` is a
  type with no type parameters, associated types or `impl` types in it,
  the checker finds the impl by `select`, the same function, on the same
  type, by which monomorphisation picks the one that runs; coherence makes
  it the only one, in any program this one is part of (an impl overlapping
  it would be rejected, wherever it is). The impl's method's body was
  checked against its own result's refinement and `ensures`, so a call of
  it may assume them. A call through a type parameter assumes the trait's,
  which have none. Parameters can't be refined, so a call through the trait
  never skips a check.
- **Lemma R2 (`Range`).** `start` and `end` are the fields, so functions of
  the value (lemma M1 of docs/21-measures.md); an integer of a `T: Int` is
  between the least `i64` and the greatest `u64` whatever `T` is.
  `next`'s body proves its refinement and `ensures` in `core`, where the
  measures are their bodies.
- **The interpreter still checks.** `t run --interp` checks each
  refinement of a type argument at each parameter, result, `let` and field
  it checks one of the whole value, of each part the value has.

## What it doesn't do

- A refinement doesn't go through a generic function: `small(n).unwrap()`
  is a `usize` nothing's known of, and so is what `?` gives. That would
  take instantiating `T` with the refined type, which is sound only for a
  type parameter generic code can't make new values of: not a `T: Int`,
  whose `+` would, nor a `T: Default`.
- Not of a struct's, a `Box`'s, a reference's or a `List`'s type
  arguments.
- `for x in a..b` knows `x < b`, but not `a <= x`: that's a fact of every
  iteration, which only a loop invariant would say.
- Trait methods still can't have refinements.

## Decisions

- Refinements of enums' type arguments only, in every place a refinement
  can be, with their meaning given by the variants' fields: they're what
  `Option`-returning functions and `for` need, and every way to make or
  read such a value is checked.
- An impl's method of a trait can refine its result and ensure more, for
  the calls known to be of it, rather than a `for`-over-a-range special
  case: so every iterator, and every other trait's impl, can say more of
  itself.
- `Range` is of integers only, with `start` and `end` measures.

## Exercises

1. Why must a refinement of `Some`'s payload be assumed only under the
   condition that the value is `Some`? Write the program that would be
   wrongly accepted if it weren't.
2. `enum Pair<T> { Both(T, T), One(T), Neither }`: what are the ways to the
   parts `Pair<{ v: u8 | v < 3 }>` is about? What would `Pair<Option<{ v:
   u8 | v < 3 }>>` be about?
3. Suppose `select` were asked of a type with a type parameter in it,
   `Range<T>`. Which impl would it find, and could monomorphisation pick
   another? What about `T` itself, with a blanket impl?
4. Sketch what it would take for `unwrap` to keep a refinement: when is
   instantiating a type parameter with a refined type sound?

## Reading

- Freeman and Pfenning, *Refinement Types for ML* (1991): refinements of
  datatypes, by the constructors a value can be.
- Vazou, Rondon and Jhala, *Abstract Refinement Types* (2013): refinements
  that are parameters of types and functions, and what parametricity
  gives them.
- Rust RFC 3245, *Refined trait implementations*: an impl's method saying
  more than its trait's, for callers that know which impl it is.
