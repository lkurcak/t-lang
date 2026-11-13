# Step 73: `for` over collections

Code: `IntoIterator` and its blanket impl in `core/iter.t`, and its impls
in `core/slice.t`, `core/array.t` and `alloc/list.t`;
`Rewrite::for_loop` in `src/check/modules.rs` and the `for` head in
`Builder::expr_into_uncoerced` (`src/borrowck/build.rs`);
`implements_at`, `select`, `project`, `assumed`, `refuted` and
`Knowable` in `src/traits.rs`; `coherence`, `orphan` and `head_of` in
`src/check/traits/impls.rs`; `coerce` and `array_method` in
`src/check/expr.rs` and `Builder::unsize`; `Hex` in `core/fmt.t`;
`escape` in `src/lexer.rs`.

Every program so far wrote `for x in l.iter()`, never `for x in &l`, and
`f(&[a, b].as_slice())` where a slice was wanted, and printed numbers in
hexadecimal with a loop of its own. Retrospective 9 found those the most
common friction left (docs/71-t-metal-findings.md,
docs/09.1-list.md). A `for` took only an iterator: going through a
reference to a list needed `IntoIterator`, whose blanket impl for every
iterator overlapped any impl for `&List<T>`, unless the checker could
tell that `&List<T>` is never an iterator. Now it can, and:

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    l.push(2);
    for x in &mut l {
        *x *= 10;
    }
    let mut sum = 0;
    for x in &l {
        sum += *x;
    }
    out.line(&sum)?; // expect: 30
    for x in [3i64, 4] {
        sum += x;
    }
    out.line(&sum)?; // expect: 37
    for x in l {
        sum -= x;
    }
    out.line(&f"{sum} is {sum:x} in hex, {sum:04x} in four digits") // expect: 7 is 7 in hex, 0007 in four digits
}
```

## `IntoIterator`

```text
export trait IntoIterator {
    type IntoIter: Iterator;
    fn into_iter(self) -> Self::IntoIter;
}

impl<I: Iterator> IntoIterator for I {
    type IntoIter = I;
    fn into_iter(self) -> I { self }
}
```

`for pat in e { body }` now means what it did with
`IntoIterator::into_iter(e)` for `e`: a `match` on it, binding the
iterator, and a `loop` calling `next` (`Rewrite::for_loop`). Core
implements it for `&[T]` and `&mut [T]`, `&[T; N]` and `&mut [T; N]`,
which give references to the values, as `iter()` and `iter_mut()` do,
and for `[T; N]` of `Copy` values, which gives copies. Alloc implements it
for `&List<T>` and `&mut List<T>`, which keeps the list's length, as
`iter_mut` says, and for `List<T>`, whose `IntoIter` takes the values out
of the list's buffer one at a time and drops what's left with it: the one
new `unsafe` block, over `RawBuf::read`. A list of linear values can be
gone through by value, but its `IntoIter` isn't `Discard`, so the `for`
can't throw it away at the end: `free_with` is still the way.

`IntoIterator` has no `Item` of its own, as Rust's has: a declaration
can't say `type IntoIter: Iterator<Item = Self::Item>` yet, which needs
equality bounds in associated types' declarations. A bound says what the
items are, `C::IntoIter: Iterator<Item = u8>`.

**What an iterator's `for` knows.** Going through `into_iter` would
forget what's known of the iterator: `for i in 0..n` knew that the range
ends at `n`, and the loop's frame said so of each `i`
(docs/52-loop-frames.md). The call's result is a new value, whose `end()`
nothing says. So where what the `for` goes through is an iterator, and the
call's type is its argument's, the borrow checker's MIR leaves the call
out, and the iterator is the one the loop goes through, as before (lemma
F1, below).

## What no impl says

Two impls overlap if their types unify (docs/06-traits.md), and bounds
didn't count, but for a sealed trait (lemma S2 of
docs/29-closed-and-sealed-traits.md). `impl<I: Iterator> IntoIterator for
I` and `impl IntoIterator for &'a List<T>` unify at `I = &'a List<T>`, and
are apart only if `&'a List<T>` is never an iterator. Saying so is
*negative reasoning*: from no impl saying it, that it doesn't hold.

That's sound only where every impl that could say it is known: no other
package can add one. By the orphan rule, an impl of trait `Tr` for a type
with head `H` is in `Tr`'s package or `H`'s. So if both are packages this
one can see, the impls `refuted` looks at are all there are, in any
program with this package in it. And if one of them is this package
itself, no other package adding an impl, of its own trait or for its own
type, can break it (Rust's *knowable* goals): coherence takes as known
only what's its package's own (decision D1).

A goal `t: Tr` is *refuted* when, of every impl of `Tr` whose type unifies
with `t`'s, a bound is refuted where they unify, recursively; never of a
type with no head, a type parameter, a projection or an opaque type; never
of a trait with the checker's own rules, `Copy` or `Eq` (but for the
sealed integer traits), nor `Ord` or `Debug` of a tuple; never past depth
8. `&Bag: Show` is refuted below because `impl Show for &T` needs `Bag:
Show`, which no impl gives; with `impl Show for Bag`, the two impls of
`Tr` would overlap, and do
(`tests/t/err_soundness_overlap_through_ref.t`).

```rust
trait Show {}

trait Tr {
    fn which(&self) -> i64;
}

impl<T: Show> Tr for T {
    fn which(&self) -> i64 { 1 }
}

impl<'a, T: Show> Show for &'a T {}

impl Show for i64 {}

#[derive(Discard)]
struct Bag {}

impl<'a> Tr for &'a Bag {
    fn which(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Bag {};
    out.line(&Tr::which(&&b))?;  // expect: 2
    out.line(&Tr::which(&&5i64)) // expect: 1
}
```

What's known of another package's type and another's trait isn't: alloc
could make `List<T>` something tomorrow without breaking itself.

```rust,error
trait Mine {}

impl<T: Iterator> Mine for T {}

impl<T> Mine for alloc::list::List<T> {}
// error: this impl of `Mine` overlaps `impl Mine for T`
```

Choosing an impl, where two headers match a type, the one whose bound is
refuted there is passed over (`select`, `project`), with any goal whose
impls are all in sight, the package's own or not: the pair was told apart
where they were checked, and that stays true (lemma N2).

A type that may still become another, with a type parameter in it, is a
question of its own. `Wrap<T>` matches only `impl<X: B> Tr for X`, but
`Wrap<Foo>`, which it may become, matches `impl Tr for Wrap<Foo>` too,
the two kept apart as `Wrap<Foo>` can't be `B`. So what the blanket impl
says `<Wrap<T> as Tr>::A` is holds only where `Wrap<T>: B` does. A
match is taken as the impl everywhere only if no other impl's type
unifies with the type, its parameters taken as variables (`alone`);
otherwise only where its bounds are proved (lemma R1). Before this step,
when bounds kept impls apart only for sealed traits, a match always was
alone. The first version of this step missed it (hole W1, never
released: `tests/t/err_soundness_blanket_partly_generic.t`).

## Blanket impls at type parameters

`fn f<I: Iterator>(it: I) { for x in it {} }` needs `I: IntoIterator`.
Resolution never applied a blanket impl to a type parameter: only its
bounds said what it implements. Nor did `T: Int` give `T: Display`, by
core's `impl<T: Int> Display for T`. Now a blanket impl applies to a type
parameter, a rigid projection or an opaque type where its bounds hold
there (rule R), and a projection on it is the impl's: `<I as
IntoIterator>::IntoIter` is `I`.

```rust
fn count<I: Iterator<Item: Discard> + Discard>(it: I) -> u32 with {panic, I::e} {
    let mut n = 0u32;
    for _x in it {
        n += 1;
    }
    n
}

fn line<T: Int>(out: &mut std::io::Stdout, x: T) -> Result<(), std::io::Error> { out.line(&x) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    line(&mut out, count(0..5i64)) // expect: 5
}
```

A bound in scope comes first, as Rust's do (decision D5): with `C:
IntoIterator` written, `C::IntoIter` is whatever it is, even if `C:
Iterator` is written too. Either is sound; this is the one Rust users
know, and what a written bound says isn't overruled by an impl.

**Hole O1.** A projection on an opaque type went through a blanket impl
whose header matched without asking whether its bounds held there. With
sealed traits (0.42.0), two impls may both match, one ruled out by a
sealed bound, and the checker took the ruled-out one's associated type,
while the program ran the hidden type's own impl. Here `f().get()` was
an `i64` to the checker and a `bool` when run, from 0.42.0 to 0.87.0;
negative reasoning would have widened it:

```rust,error
sealed trait Small {}

impl Small for u8 {}

trait Tr {
    type A;

    fn get(&self) -> Self::A;
}

impl<T: Small> Tr for T {
    type A = i64;

    fn get(&self) -> i64 { 5 }
}

#[derive(Discard)]
struct Foo {}

impl Tr for Foo {
    type A = bool;

    fn get(&self) -> bool { true }
}

fn f() -> impl Tr + Discard { Foo {} }

fn main() {
    let x: i64 = f().get(); // error: expected `i64`, found `<impl Tr + Discard as Tr>::A`
    let _y = x + 1;
}
```

Rule R is what fixes it: `f()`'s type says it's `Tr`, so the projection
stays rigid (R2), as it would if no blanket impl's bound held.

## References belong to what they point to

The orphan rule now looks through `&` and `&mut`, as Rust's
`#[fundamental]` does: `impl IntoIterator for &'a List<T>` is alloc's to
write, as `List` is, and a program may implement a trait for `&Bag` or
`&mut Bag` of its own `Bag`. Before, every reference was core's.

## Arrays as slices

Where a `&[T]` is wanted, a `&[T; N]` is coerced to it (decision D4), as
deref coercions are: by the checker, recorded, and written out by the
borrow checker as the call a program could write, `as_slice()` or
`as_mut_slice()` (`Builder::unsize`). So what's known of the slice is
what that says, that it's as long as the array:

```rust
fn third(s: { s: &[i64] | s.len() == 3 }) -> i64 { s[2] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1, 2, 3];
    out.line(&third(&a))?; // expect: 3
    let s: &[u8] = &[7, 8];
    out.line(&s[1]) // expect: 8
}
```

## Hexadecimal and long strings

A hole `{x:x}` writes an integer in lowercase hexadecimal, and `{x:08x}`
with at least 8 digits, 0s in front, a subset of Rust's (decision D3). A
negative value is written as its two's complement bits, as many as its
type has: `-1i8` is `ff`. It's `core::fmt::hex(&x, 8)`, whose `Hex<T>`
is `Display` for `T: Int`, in T.

A `\` that ends a line of a string, a byte string or a format string goes
on to the next, without the line break or the blanks that start the next
line, as in Rust:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 255u8;
    out.line(&f"{n} is {n:x}, \
                or {n:04x}") // expect: 255 is ff, or 00ff
}
```

## Why it's sound

- **N1, refuted is false everywhere.** Every impl that could prove an
  instance of a knowable goal `t: Tr` is in `Tr`'s package or the
  package of `t`'s head (the orphan rule, references looked through),
  both in this package's closure, and impls are checked once all of
  theirs are collected. An impl proves an instance of `t` only at an
  instance of the unifier of its type and `t`'s, where the bound refuted
  at the unifier is refuted too, by induction on depth. So no instance of
  `t` implements `Tr`, in any program with this package in it.
- **N2, refuted stays refuted.** The impls that could prove an instance
  `σt` are among those for `t`, and their bounds at `σt`'s unifier are
  instances of those at `t`'s: so the same tree refutes `σt`, no deeper.
  Coherence's verdict at two impls' unifier holds at every type both
  match: at most one impl is found for any type, the same one by the
  checker and by monomorphisation.
- **N3, dependencies can't break it.** Coherence refutes only goals of
  its package's own, by trait or head. Another package adding an impl of
  its own trait or for its own type adds none of those; only adding a
  blanket impl to a trait can, as in Rust, where that's a breaking change.
- **O', the orphan rule still makes coherence local.** An impl for `&X`
  is in its trait's package or `X`'s head's, so two in packages that don't
  depend on each other can't be for the same type: each names a type of
  its own, or is its trait's package.
- **R1, a blanket impl at a type parameter is the one.** Whoever calls the
  function proved its bounds at the types it calls it at, so the blanket
  impl's bounds hold there too, being proved from those. Any other impl
  that matches such a type overlaps the blanket impl, so one of the two
  was refuted where they unify (coherence), and so at the type (N2); the
  blanket impl's bounds hold, so it's the other. Monomorphisation finds
  the blanket impl, and its associated type there is what the checker's
  was, with the type for the parameter.
- **R2, what a bound says first.** A written bound's projection stays
  rigid, which is less known, never wrong: it's an associated type that
  monomorphisation works out at each type, as for any rigid projection.
- **F1, a `for` over an iterator goes through it.** If `t: Iterator`
  where the `for` is, every impl of `IntoIterator` but the blanket one
  was told apart from it by refuting `t: Iterator` where they'd unify, so
  the blanket impl is the one, at `t` and at every instance (R1), and its
  `into_iter` is `self`: leaving the call out runs the same.
- **C1, a coercion is a call.** An array coerced to a slice is the
  `as_slice()` a program could write, at the same place: the borrow
  checker, the refinement checker and the backends see that call.

## What it doesn't do

- **Arrays of linear values by value.** Taking values out of an array one
  at a time leaves it part full, which nothing tracks (decision D2).
- **`Item` of `IntoIterator`.** It waits for equality bounds in
  associated types' declarations.
- **Negative reasoning about the checker's own traits.** `Bag: Copy` is
  never refuted, though `Bag` derives nothing: the checker's rules prove
  `Copy` of more than impls do, and saying exactly when is a list to keep
  in step with them.
- **Hexadecimal in uppercase, `{x:#x}`, or widths with spaces.** Not asked
  for yet.
- **A hex hole of a non-integer passed straight to a call.**
  `out.line(&f"{b:x}")` of a `bool` reports `line`'s bound rather than
  the hole's, as the call's obligation is proved first.

## Decisions

- **D1: coherence reasons only about the package's own goals.** Not about
  every goal whose impls are in sight, which another package could break
  by adding an impl for its own type.
- **D2: by value, `List<T>` and arrays of `Copy` values.** References to
  lists, slices and arrays for the rest.
- **D3: `{x:x}` and `{x:0Nx}`**, Rust's syntax, a negative value as its
  bits.
- **D4: the array coercion wherever deref coercion applies.** Not only
  for `&[..]` written in place.
- **D5: blanket impls at type parameters, with bounds in scope first.**
  Rather than a special case for `for` alone.

## Exercises

1. Why must coherence's refutation stop at a type parameter, even one
   whose package is this one's? (Hint: what can a caller choose it to
   be?)
2. Let coherence refute goals of any package it can see, not only its
   own. Write two packages, the second depending on the first, where a
   new version of the first that adds an impl breaks the second.
3. Make rule R's projection skip the check that the blanket impl's bounds
   hold. Which test fails, and what does the program it checks print when
   run?
4. Leave the `for` head's call in for iterators too. Which proofs about
   `for i in 0..n` stop going through, and why?
5. `Bag: Copy` is never refuted. Sketch what `refuted` would need to know
   to refute it soundly, given `#[derive(Copy)]` and tuples' rules.

## Reading

- Rust RFC 1023, *Rebalancing coherence* (2015): the orphan rules,
  `#[fundamental]` types, and which impls a crate may add without
  breaking those that depend on it.
- Rust RFC 2451, *Re-rebalancing coherence* (2018).
- The rustc dev guide, *Coherence*: the overlap check, and which goals
  it may treat as known not to hold.
- Peyton Jones, Jones and Meijer, *Type classes: an exploration of the
  design space* (1997): overlapping instances and coherence.
