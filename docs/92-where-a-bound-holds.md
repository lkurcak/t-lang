# Step 92: where a bound holds

Code: `Checker::guard_pred` and `Checker::effect_guards`
(`src/check/refine.rs`), which check what's written;
`Frame::invariant` (`src/check/refine/mir.rs`), which says which clauses
of an invariant a walk has; `Checker::scenario_walks`, which walks a body
again under a bound; `Checker::unguarded` and `Checker::barred`
(`src/check/effects.rs`), which decide what a call and a body have; and
`Cx::pred` and the effects of each copy (`src/mono.rs`), which write what
holds in each copy, for the interpreter.

```rust
fn kept(a: &[u8]) -> usize with {} { a.iter().filter(|x| **x > 1).skip(1).count() }

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&kept(&[1, 2, 3, 4]))?; // expect: 2
    Ok(())
}
```

`count` adds one for each value, and a sum can overflow: so `count` said
`with panic`, of every iterator. Of a slice's it can't: there are no more
values than the slice is long. Step 88 gave iterators laws, and step 90
effects that hang on values, but neither could say this, as what makes
`count` safe isn't a value it's given. It's a *bound*: that the iterator
is one that says how many values it has left.

So three things can now be said only where a bound holds, which is what
this step and the next are about:

- a clause of a loop's invariant, `invariant p where T: Trait`;
- an effect, `with panic unless T: Trait`;
- and an invariant of a struct, a field's refinement
  (docs/93-invariants-where.md).

All three are checked the same way: the body is walked again *with the
bound*, as a law's is (docs/88-iterator-laws.md).

## Bounded iterators

```rust
#[derive(Discard)]
struct Down { n: usize }

impl Iterator for Down {
    type Item = usize;
    effect e = {};

    fn next(&mut self) -> Option<usize> {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

impl BoundedIterator for Down {
    #[measure]
    fn most(&self) -> usize { self.n }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&(Down { n: 7 }).filter(|n| *n > 3).count())?; // expect: 3
    Ok(())
}
```

What the programs written so far count is seldom an iterator that knows
how many values it has: it's what `filter` keeps, or a string's `chars`.
Those know how many they have *at most*. So the bound isn't
`ExactSizeIterator`. It's a trait under it:

```text
export trait BoundedIterator: Iterator {
    #[measure]
    fn most(&self) -> usize;

    law fn next(&mut self) -> { r: Option<Self::Item> | !r.is_some() || self.most() < old(self.most()) }
    with Self::e
    ensures
        self.most() <= old(self.most());
}

export trait ExactSizeIterator: BoundedIterator {
    #[measure]
    fn len(&self) -> usize { self.most() }

    law fn next(&mut self) -> { r: Option<Self::Item> | r.is_some() == (old(self.len()) > 0) }
    with Self::e
    ensures
        self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0);
}
```

`most` never grows, and each value given takes at least one off it. An
`ExactSizeIterator` is one whose `most` is exact: its `len` is `most`, a
measure with a body in the trait, which no impl can change
(docs/86-trait-contracts.md), so an impl of it is empty, and says only
that the law holds. Which is checked, against the body of `next`.

Slices', arrays' and lists' iterators are both, and so are `map`,
`enumerate`, `rev`, `zip` and `skip` of ones that are. `filter` of a
bounded one is bounded, and never exact.

## A clause of an invariant

`Filter`'s `next` loops until its predicate keeps a value. That it takes
at least one off `most` needs an invariant: that `iter.most()` is at most
what it was. But `Filter` is an iterator of any iterator, and `most` is a
measure only of a bounded one. So the clause says where it holds:

```text
fn next(&mut self) -> Option<I::Item> with {I::e, px} {
    loop invariant self.iter.most() <= old(self.iter.most()) where I: BoundedIterator {
        match self.iter.next() {
            Some(x) => {
                if (self.p)(&x) {
                    return Some(x);
                }
            },
            None => return None,
        }
    }
}
```

A clause with a `where` is checked as a predicate where the bound holds,
so it can call the bound's trait's measures. A walk of the body *says* it,
assuming it at the loop's head and proving it at each way back there
(docs/54-loop-invariants.md), only if the walk's bounds prove the bound:
the function's own, or those a law is checked under. The walk of
`Filter`'s `next` for every iterator doesn't have the clause at all. The
one that checks `BoundedIterator`'s law, of `Filter<I, P>` where `I:
BoundedIterator`, has it, and needs it.

A loop can have several clauses, each after its own `invariant`, and
`t fmt` puts each on a line of its own when they don't fit on the loop's:

```rust
use core::num::greatest;

fn capped<I: Iterator + Discard>(it: I, cap: usize) -> { n: usize | n <= cap }
with I::e
where
    I::Item: Discard,
{
    let mut n: usize = 0;
    for _x in jt: it
        invariant n <= cap
        invariant n + jt.most() <= greatest::<usize>() where I: BoundedIterator
    {
        if n < cap {
            n = n + 1;
        }
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&capped([1u8, 2, 3].iter(), 2))?; // expect: 2
    Ok(())
}
```

Two more things an invariant can say came with this:

- **`old(x)`**, what a parameter of the function was when it was called:
  `self.iter.most() <= old(self.iter.most())`. Only of parameters: a local
  had no value then. A `mut` parameter's is what it was given, as in a
  condition (docs/90-conditional-effects.md).
- **`least::<T>()` and `greatest::<T>()`** of an integer type, in any
  predicate: `n + it.most() <= greatest::<usize>()`. The logic knew them
  already (docs/12-bits.md); now they can be written.

## An effect, unless

```rust
trait Value {
    fn value(&self) -> u8;
}

/// One whose value has room for one more: a law of `value`.
trait Low: Value {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Small { n: { v: u8 | v < 100 } }

impl Value for Small {
    fn value(&self) -> u8 { self.n }
}

impl Low for Small {}

fn bump<T: Value + Discard>(x: T) -> u8 with panic unless T: Low {
    let v = x.value();
    x.discard();
    v + 1
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&bump(Small { n: 5 }))?; // expect: 6
    Ok(())
}
```

`with panic unless T: Low` says `bump` may panic, but not where `T` is
`Low`. `unless` is a word only there, after an effect. It's for `panic`,
`alloc` and `global`, as a condition is, and an effect has one or the
other, not both. The bound is one trait, of a type parameter, `Self`, or
an associated type of one.

**A call** has the effect wherever the bound isn't known to hold of its
type arguments: of a type that doesn't meet it, or a type parameter with
no such bound. Where it's known to, it has none. Knowing more can only
take the effect away, so adding an impl never breaks a caller.

**The body** is checked twice. Once as ever, where it may have the effect.
And once with the bound, where nothing in it may: an operator must be
proved not to overflow there, as `v + 1` is by `Low`'s law; a call must
have no such effect there, by its condition or its own `unless`; a
`panic(..)` mustn't be there at all. Clauses of invariants that hang on
the bound are said in that walk, which is what `count` needs:

```text
fn count(self) -> usize
with {Self::e, panic unless Self: BoundedIterator}
where
    Self: Discard,
    Self::Item: Discard,
{
    let mut n = 0;
    for _x in it: self
        invariant n + it.most() <= greatest::<usize>() where Self: BoundedIterator
    {
        n = n + 1;
    }
    n
}
```

```rust,error
trait Small {
    fn size(&self) -> u8;
}

fn bump<T: Discard>(x: T, n: u8) -> u8 with panic unless T: Small {
    x.discard();
    n + 250
}

fn main() {}
// error: `+` may panic where `T: Small`, but `bump` says `with panic unless T: Small`
```

**The type** of the function is what it was, `fn(T) -> u8 with panic`:
used as a value it has the effect, whatever its type arguments, as what
calls it knows only its type. A closure's effects are what its body has
without the bound, so a call of one that may panic is a site that may,
with it too.

**A trait's method** can say it, of `Self`. Its default body is checked
with `Self: Trait`. An impl's own method in its place has the trait's
method's effects, as ever (docs/70-bounds-in-impls.md), and may have the
effect only if its type can't meet the bound: if some impl of the bound's
trait may be for it, it can't, as a call through the trait there has
none. "Its type" is every type the impl is for: an `impl<T> Bump for
W<T>` is for `W<u8>` too, so an `impl Small for W<u8>` is enough to
take the effect away from its `bump`, and so is an impl another package
could write, of `&T` for its own `T`.

```rust,error
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {
    fn size(&self) -> u8 { self.n }
}

trait Bump {
    fn bump(&self) -> u8 with panic unless Self: Small { 1 }
}

impl Bump for A {
    fn bump(&self) -> u8 with panic { self.n + 250 }
}

fn main() {}
// error: `+` may panic where `A: Small`, but `bump` says `with panic unless A: Small`
```

**Beside another effect**, an `unless` takes out only what it's on. `count`
says `with {Self::e, panic unless Self: BoundedIterator}`: of a bounded
iterator whose `next` may panic, `Self::e` is `panic`, and the call has
it. A condition couldn't be beside an associated effect
(docs/90-conditional-effects.md); an `unless` can, as each call is given
what the rest of the signature is there.

## Why it's sound

- **Lemma B1 (a walk under a bound).** What a walk of a body proves under
  bounds `E` and a bound `B` holds of every copy of the body whose types
  meet `E` and `B`: the walk used only what those types have in common
  (lemma L1 of docs/88-iterator-laws.md).
- **Lemma B2 (bounds only grow).** A bound proved from the bounds in scope
  holds of every copy made where those do: impls are found the same way
  when copies are made, and none is ever taken away. So what's decided
  where a bound is known to hold is right in every copy, and what's
  decided where it isn't is only less.
- **Lemma B3 (clauses).** A clause of an invariant is assumed only by a
  walk that also proves it, at the loop's start and at each way back to
  its head. A walk that doesn't say it claims nothing of it.
- **Lemma B4 (bodies).** A function `g` that says `a unless B` is walked
  under `B`, and after that walk no site of its body has `a`: each
  operator is proved, each call's `a` is ruled out by the callee's
  condition or by the callee's own `unless`, whose bound follows from
  `B` and `g`'s bounds, and nothing else is left. A closure's effects and
  an effect parameter's are counted as they are without `B`.
- **Lemma B5 (calls).** A call of `h` at type arguments `σ` has every
  effect of `h`'s but each `a unless B` whose `B[σ]` is proved where the
  call is, and whose `a` isn't among what the rest of `h`'s effects are
  at `σ`.
- **Lemma B6 (every body).** Each body a call of a trait's method with an
  `unless` can run is under B4: the trait's default is walked under
  `Self: B`, in the trait's package; an impl's own method has no `a` at
  all if any impl of `B`'s trait may be for an instance of its type, as
  `W<u8>` is of `W<T>`. That's `B` not being *refuted* of its type
  (lemma N1 of docs/73-for-over-collections.md): every impl that could
  be for an instance is in a package loaded where the impl is, by the
  orphan rule, and its type unifies with the impl's. A copy of the
  method runs at an instance; if `B` holds there, the method was checked
  without `a`. A trait whose methods' bodies are written for it, derived
  or built in, can't say `unless`.
- **Lemma B7 (the stack).** When effect `a` happens, every function on
  the stack says `a`, or says it with a condition that held (lemma C5 of
  docs/90-conditional-effects.md), or says `a unless B` in a copy whose
  types don't meet `B`. For a copy whose types meet `B`: by B4 no site of
  its body has `a` under `B`, so the site the stack goes through is a
  call that had no `a` by B5, whose callee's copy, by B2, is one whose
  own bound holds, and so on up: and the top of the stack is a site that
  has `a`. Which no such copy has.
- **Lemma B8 (the oracle).** Monomorphisation leaves out of each copy's
  effects each `a unless B` whose bound holds of the copy's types, and
  writes each invariant's clauses that are said there. The interpreter
  checks the effects of every function on the stack at each effect, and
  the clauses at each loop's head, as it did.

## What it doesn't do

- **`enumerate` still says `with panic`.** Its index is a field of
  `Enumerate`, and that it can't overflow is an invariant of the struct,
  where its iterator is bounded (docs/93-invariants-where.md), which its
  associated effect must then say: step 94.
- **`chars` isn't bounded yet**, nor a range: `Chars`'s `next` needs an
  invariant of its own, and `Range<usize>` an impl for one type of a
  generic one (docs/88-iterator-laws.md).
- **`count` has no law yet.** That it's at most `most`, and `len` of an
  exact iterator, needs the loop to say what it was given, which `old`
  now can: when a use wants it.
- **One bound, on a type parameter.** Not two, nor one on a type that's
  only partly known, `Skip<I>`, whose bound hangs on `I`'s.
- **A closure in a body with an `unless`** has its effects whatever the
  bound, so `self.fold(0, |n, _x| n + 1)` wouldn't do for `count`: the
  loop is written out.
- **An impl's own method** in place of a default with an `unless` isn't
  walked under the bound: it just can't have the effect, of a type that
  may meet it, at any of its instances. Until 0.106.1 only the impl's
  type itself was asked of, by matching impls against it, so an `impl<T>
  Bump for W<T>` could panic though `impl Small for W<u8>` was there,
  and a call at `W<u8>` had no `panic` (retrospective 18).

## Decisions

- **D1: clauses, invariants and effects that hang on a bound**, checked by
  walking the body again with it. Bounds as propositions inside one walk
  would put a condition on every law and every measure; the walk again is
  what laws already do.
- **D2: `panic unless B`.** A bound can only ever be used one way: proved,
  it takes the effect away. `if !(B)` would say a bound is a condition
  like any other, to be negated and joined, which it can't be.
- **D3: `BoundedIterator`, with `ExactSizeIterator` under it.** What's
  counted is mostly not exact.
- **D4: `invariant p where B`**, and several `invariant`s for one loop.
- **`Skip`'s `next` no longer counts a `None` as skipped.** An iterator
  that isn't exact can give `None` with its `most` unchanged, and `most`
  of the `Skip` would grow. Of every iterator in the library it's the
  same as before.

## Exercises

1. `Filter` is bounded. Why can't it be exact? What would its `len` be?
2. Write `take(n)`: an iterator of at most `n` of another's values. It's
   bounded whatever the other is. What's its `most`, and what does its
   law's proof need?
3. `count`'s invariant is `n + it.most() <= greatest::<usize>()`. Why does
   it hold at the start? Why can't it be written without the `where`?
4. Write a default method that says `with alloc unless Self: Copy`, and
   an impl for a `Copy` type with a method of its own that allocates.
   What does the checker say, and why must it?
5. In `Checker::unguarded`, take out the check of the rest of the
   callee's effects. Which test fails, and does the interpreter notice?

## Reading

- Wadler and Blott, *How to make ad-hoc polymorphism less ad hoc* (1989):
  a bound as evidence passed to the code, which is what a walk under a
  bound assumes it has.
- Chakravarty, Keller and Peyton Jones, *Associated type synonyms*
  (2005), and Rust's specialization RFC 1210, for what goes wrong when
  code may depend on a bound *not* holding: here it never can.
- Leino, *Dafny: an automatic program verifier* (2010), on loop
  invariants and `old`.
- Rust's `Iterator::size_hint` and `TrustedLen`: an upper bound that code
  may read, and an `unsafe` trait for one it may count on.
