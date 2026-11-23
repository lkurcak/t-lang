# Step 93: an invariant where a bound holds

Code: `Checker::fields_now` (`src/check/refine.rs`), which checks a
field's refinement that ends `where`; `Walk::refined_fields`, the one
place the walk reads a struct's refinements, which says which a value has
where the walk is; `Checker::field_scenarios`, which walks a body again
under a bound; `Checker::scoped_params`, for a lend to a trait's method;
and `Cx::pred` (`src/mono.rs`), which writes what holds in each copy, for
the interpreter.

```rust
use core::num::greatest;

/// Each value of `iter` with its index.
#[derive(Discard where I: Discard)]
struct Counted<I> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() where I: BoundedIterator },
}

impl<I: Iterator> Counted<I> {
    fn new(iter: I) -> Counted<I> { Counted { iter, next: 0 } }

    fn step(&mut self) -> Option<(usize, I::Item)> with {I::e, panic unless I: BoundedIterator} {
        match self.iter.next() {
            Some(x) => {
                let i = self.next;
                self.next = i + 1;
                Some((i, x))
            },
            None => None,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    let a = [5u8, 6, 7];
    let mut c = Counted::new(a.iter());
    let _first = c.step();
    if let Some((i, x)) = c.step() {
        out.line(&f"{i} {x}")?; // expect: 1 6
    }
    Ok(())
}
```

`step` adds one to an index, and a sum can overflow. Of a slice's iterator
it can't: the index and what's left to give are never more than the slice
is long. That's a fact about every `Counted` of such an iterator, an
*invariant* of the struct, and a struct's invariants are its fields'
refinements (docs/08.1-field-refinements.md). But `Counted` is a struct of
any iterator, and `most` is a measure only of a bounded one
(docs/92-where-a-bound-holds.md). So the refinement says where it holds.

This is the third thing that can be said only where a bound holds, after a
clause of a loop's invariant and an effect, and it's checked the same way:
the body is walked again *with the bound*.

## A refinement that ends `where`

A struct's field's refinement can end `where P: Trait`, with `P` a type
parameter of the struct, or an associated type of one. It's what a value
of the struct has *where the bound holds of its types*: a `Counted<Iter<'_,
u8>>` has it, and a `Counted<Ones>` of an iterator that never ends doesn't.

The predicate is checked as one where the bound holds, so it can call the
trait's measures. Only a field's own refinement can end so: not one of
what's in the field (`Option<{ v: u8 | .. }>`), which is its type's to say
of every value of it, nor a parameter's, a result's or a `let`'s.

```rust,error
fn f<T: Discard>(x: { v: u8 | v > 0 where T: Copy }, t: T) -> u8 {
    t.discard();
    x
}

fn main() {}
// error: only a struct's field's own refinement can end `where`
```

## Where it's known

A body knows the refinement of a value where it knows the bound: by its
own bounds, or because the type is one that meets it.

```rust,error
use core::num::greatest;

struct Counted<I> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() where I: BoundedIterator },
}

fn after<I: Iterator>(c: &Counted<I>) -> usize with {} { c.next + 1 }

fn main() {}
// error: `+` may panic, but `after` doesn't say `with panic`
```

With `I: BoundedIterator`, `after` is accepted: `next` is then less than
the greatest `usize`, unless `iter` has nothing left, which the checker
can't rule out, so in fact it still isn't. The index of a value `step` just
gave is what's known to have room.

## Where it's kept

Everything that makes a `Counted`, or changes a field its refinement is
about, must keep it, as ever. What's new is *where*: at each type the bound
may hold of. For a body that has a `Counted<I>` of its own type parameter,
with no such bound, that's some of its copies and not others. So the body
is checked twice: once as it is, where the struct has no such refinement;
and once with `I: BoundedIterator`, where it has, and every rule of a
refinement applies: a literal must have it, a write must keep it, a field
it names can be lent `&mut` only straight to a call, and is proved again
after.

```rust,error
use core::num::greatest;

struct Counted<I> {
    iter: I,
    next: { n: usize | n + iter.most() <= greatest::<usize>() where I: BoundedIterator },
}

fn late<I: Iterator>(iter: I) -> Counted<I> { Counted { iter, next: 9 } }

fn main() {}
// error: can't prove `n + iter.most() <= greatest::<usize>()` for field `next` of `Counted`
```

The error says which walk found it: `where I: BoundedIterator, which the
refinement of field next of Counted hangs on`. In `Counted::new` above,
`next: 0` has it with the bound, as a measure's result is never more than
its type holds; and in `step`, `i + 1` is proved there, by
`BoundedIterator`'s law that a value given takes one off `most`, which is
also what makes the write `self.next = i + 1` keep the refinement.

The bound needn't be on a type parameter. Of `Counted<Skip<J>>` it's
`Skip<J>: BoundedIterator`, which holds where `J` is bounded, and of a
`Counted` of an `impl Iterator` it's a bound on a type nobody can name:
each is walked with that bound taken as given. Of a type with no parameter
in it, the bound holds or it doesn't, now and in every program: an impl of
a trait for a type is in the package of one or the other
(docs/06e-core.md), and both are in sight.

## Two things that were missing

- **A struct's bounds hold in its fields' refinements.** `struct
  Numbered<I: BoundedIterator> { iter: I, next: { n: usize | n +
  iter.most() <= greatest::<usize>() } }` says the same of a struct that's
  only of bounded iterators, with no `where`. The measure wasn't found
  there before.
- **A lend to a trait's method.** A field a refinement names can be lent
  `&mut` straight to a call whose parameter can't outlive it, and is proved
  again when the call returns (docs/30-postconditions.md). That was so of a
  function's parameter, and wasn't of a trait's method's with no
  refinement of its own: so `self.iter.next()` was refused. Whether a
  reference can outlive a call is its signature's to say, and an impl's
  method has its trait's.

## `Enumerate`

`core::iter::Enumerate` now says the same as `Counted`. Its `next` still
says `with panic`: that it can't, of a bounded iterator, is what its
*associated effect* would have to say, `effect e = {I::e, panic unless I:
BoundedIterator}`, and an associated effect is part of a type. That's
step 96 (docs/96-associated-effects-unless.md), since which it does.

## Why it's sound

A value of `S<τ>` *has* the refinement if `τ` meets the bound in the
program that runs.

- **Lemma I1 (known).** A walk assumes the refinement of a value only if
  its bounds prove the bound of the value's types: which then holds of
  every copy of the body (lemma B2 of docs/92-where-a-bound-holds.md).
- **Lemma I2 (kept, where known).** Where a walk has the refinement, it
  proves it at everything that makes the struct or changes what it's
  about, as for any refinement (docs/08.1-field-refinements.md,
  docs/30-postconditions.md, docs/66-one-walk.md).
- **Lemma I3 (kept, where it may hold).** A walk that reads the
  refinements of a struct at types whose bound it can't prove, and that
  have a type parameter or an opaque type in them, leaves the bound to be
  walked under. That walk has the refinement, so by I2 and lemma B1 of
  docs/92 the body keeps it in every copy whose types meet the bound. One
  bound at a time: a copy that meets two has each kept by its own walk,
  which assumed no more than the copy has. Every read leaves it, not only
  what makes or changes a value: there's no list of those to be wrong.
- **Lemma I4 (never).** A bound of types with no parameter and no opaque
  type in them is decided when it's asked, and stays so: an impl that
  would make it hold is in the trait's package or the type's head's (the
  orphan rule), which are the body's own or ones it depends on, and whose
  impls are all collected. So a value at types that don't meet it never
  has the refinement, and there's nothing to keep.
- **Lemma I5 (every value).** By induction on the steps of a run: a step
  that makes or changes a struct at types that meet the bound is in a copy
  that I2's walk or I3's covers, and those walks assume the refinement
  only of values that were there before the step.
- **Lemma I6 (a struct's bounds).** A struct's bounds hold of every type
  it's used at: each use proves them. A trait's measure is a function of
  the value it's of (lemma H1 of docs/86-trait-contracts.md), so what a
  refinement says with one changes only when a field is written.
- **Lemma I7 (a trait's method's lend).** A parameter is call-scoped by
  its lifetime's place in the signature (docs/30-postconditions.md), and
  an impl's method's signature is its trait's, lifetimes and their bounds
  too (docs/70-bounds-in-impls.md). An associated type in it is a
  function of `Self`, which doesn't hold the reference's lifetime.
- **Lemma I8 (the oracle).** Monomorphisation writes each copy of the
  struct with the predicate, where the bound holds of the copy's types,
  or `true`. The interpreter checks it when one is made, written or read.

## What it doesn't do

- **`enumerate` still says `with panic`**, as above: until step 96.
- **One bound, and the whole refinement hangs on it.** A field can't have
  one part that always holds and one that doesn't, nor a refinement of
  what's in it beside.
- **Only a struct's field.** A parameter's refinement where a bound holds
  would be a precondition only of some callers; a variant's field's, when
  a use wants it.
- **A read walks again too.** A body that only reads a struct at types
  that may meet the bound is walked with it, for nothing: it's one more
  walk of a small body, and telling reads from writes is where a mistake
  would be a hole.
- **A bound that can't hold of any type, but has a parameter in it**
  (`Wrap<J>: Trait`, with no impl for any `Wrap`) is walked with all the
  same, and what the walk can't prove is an error, of copies there will
  never be.

## Decisions

- **D1: in the field.** `{ n: usize | p where I: Trait }`, not an
  `invariant` after the struct. A struct's invariants already are its
  fields' refinements: which fields one is about, who can see it, what a
  write must prove and what can be lent are all decided there.
- **D2: this step, a retrospective, then associated effects.** An
  associated effect that hangs on a bound changes what an effect in a
  type means where the bound isn't known, which is its own step.

## Exercises

1. Write `Counted::new` with `next: 1`. Which walk refuses it, and of
   which iterator is it wrong?
2. `fn refill<I: Iterator>(c: &mut Counted<I>, more: I) -> I` puts `more`
   in `c.iter` with `core::mem::replace` and gives back what was there.
   Why is it refused? What would it have to do first to be accepted?
3. Why is `Counted { iter: Ones {}, next: greatest::<usize>() }` accepted,
   of an iterator `Ones` that isn't bounded? Why can no library that uses
   yours make it wrong?
4. Give `Counted` a method `rest(&self) -> usize` for `I:
   ExactSizeIterator` that says how many values are left. What does its
   body know of `self.next`?
5. In `Walk::refined_fields`, record a bound only if its type is a type
   parameter. Which tests fail, and does the interpreter notice?

## Reading

- Barnett, DeLine, Fähndrich, Leino and Schulte, *Verification of
  object-oriented programs with invariants* (2004): when an invariant may
  be broken, and who must restore it.
- Vazou, Rondon and Jhala, *Abstract refinement types* (2013): an
  invariant that depends on how a type is used, by a parameter of the
  refinement where T has a bound.
- Rust's `TrustedLen` and specialization (RFC 1210): code that's right
  only for some type arguments, chosen by a bound, and why choosing by a
  bound that may *not* hold is unsound. Here a bound only ever adds what's
  known.
