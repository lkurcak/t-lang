# Step 96: associated effects that hang on a bound

Code: `ImplDef::assoc_unless` and `project`, `upper` and `guarded` in
`src/traits.rs`, which decide an impl's associated effect where its
bounds say; `Projections::guarded_set`, which takes a set of effects
apart into what each copy has; in `src/check/effects.rs`
`Checker::effect_bound` and `body_effects`; `Checker::scenario_guards`
(`src/check/refine.rs`), which walks a body again under the bounds; and
`same_signature` (`src/check/traits/impls.rs`), which compares an impl's
method with its trait's.

`enumerate` of an iterator that says how many values it has at most
can't panic, and says so:

```rust
fn second(s: &[u64]) -> u64 with {} {
    let mut t = 0;
    for (i, x) in s.iter().enumerate() {
        if i == 1 {
            t = *x;
        }
    }
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {} {
    let mut out = os.stdout;
    out.line(&second(&[4, 5, 6]))?; // expect: 5
    Ok(())
}
```

Its index can't pass the greatest `usize` where the iterator is bounded,
an invariant of `Enumerate` since step 93 (docs/93-invariants-where.md).
What was missing was a way for `Enumerate`'s `next` to say that its
`panic` hangs on the same bound, as `count` says it of itself (docs/92-where-a-bound-holds.md):
`next` is the trait's method, whose effects are the impl's associated
effect, `e`, which said `{I::e, panic}` of every iterator.

## An atom that hangs on a bound

An impl's associated effect can say an atom `unless` a bound holds, as a
function's effects can:

```text
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    effect e = {I::e, panic unless I: BoundedIterator};
    ..
}
```

The atoms are `panic`, `alloc` and `global`, each said once; the bound is
one trait, of a type parameter of the impl or an associated type of one.
Not of `Self`, where that's `Enumerate<I>`, only partly a parameter.

`<X as Iterator>::e`, of a type `X` this impl is for, has `panic` where
`X`'s iterator isn't bounded. Where that's decided, it's worked out as
ever: of `Enumerate<Iter<'_, u64>>`, `Iter`'s `e`, which is none; of
`Enumerate<Chars<'_>>`, that and `panic`; in code generic in `I:
BoundedIterator`, `I::e`. Where it isn't decided, in code generic in an
`I: Iterator` that may or may not be bounded, it stays itself,
`<Enumerate<I> as Iterator>::e`, which is what a type parameter's
associated effect always is there (docs/68.1-effect-bindings.md):

- **Within a set**, it's at most its binding with every atom: a call of
  `next` there is within `{I::e, panic}`.
- **A set within it** must be within it as it is, never within that
  binding, which it isn't where `I` is bounded. A function that panics
  isn't a `fn() with <Enumerate<I> as Iterator>::e`:

```rust,error
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic unless T: Low { self.x.value() }
}

fn boom() -> u8 with panic { panic("boom") }

fn pick<T: Val>(_w: &W<T>) -> fn() -> u8 with <W<T> as Go>::e { boom }

fn main() {}
// error: expected `fn() -> u8 with <W<T> as Go>::e`, found `fn() -> u8 with panic`
```

## What says it

The impl's own method says it as the trait's does, `with Self::e`, or
spelled out, `with {I::e, panic unless I: BoundedIterator}`: an impl's
method can say `unless` now, of what it does, and has at most what its
trait's method has there, as each copy has it. Where the impl's `e` has
`panic` only unless `T: Low`, so must the method:

```rust,error
trait Val {
    fn value(&self) -> u8;
}

trait Low: Val {}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with panic { self.x.value() + 1 }
}

fn main() {}
// error: `go` may panic where `T: Low`, but `Go`'s `go` can't there, as this impl's associated effect says; so it can't either: say `with panic unless T: Low`
```

Either way the body is checked twice, as a function's with an `unless`
is: once as it is, where it may panic, and once with `T: Low`, where it
may not. So is any function that says such a projection where it isn't
decided: `with Self::e` inside the impl, where `Self::e` stays itself
too, or `with <W<T> as Go>::e`. With the bound, a call of what's
generic in it is worked out again: in a function that says `with {I::e,
panic unless I: BoundedIterator}`, a call of `Enumerate`'s `next` has no
`panic` there.

```rust
trait Val {
    fn value(&self) -> u8;
}

/// One whose value has room for one more: a law of `value`.
trait Low: Val {
    law fn value(&self) -> { r: u8 | r < 255 };
}

#[derive(Discard)]
struct Lo { n: { v: u8 | v < 100 } }

#[derive(Discard)]
struct Any { n: u8 }

impl Val for Lo {
    fn value(&self) -> u8 { self.n }
}

impl Low for Lo {}

impl Val for Any {
    fn value(&self) -> u8 { self.n }
}

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Val> Go for W<T> {
    effect e = panic unless T: Low;

    fn go(&self) -> u8 with Self::e { self.x.value() + 1 }
}

fn low() -> u8 with {} { (W { x: Lo { n: 7 } }).go() }

fn any<T: Val>(w: &W<T>) -> u8 with <W<T> as Go>::e { w.go() }

fn last_index<I: Iterator<Item = u64> + Discard>(it: I) -> usize
with {I::e, panic unless I: BoundedIterator}
{
    let mut n = 0;
    for (i, _x) in it.enumerate() {
        n = i;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&low())?;                                    // expect: 8
    out.line(&any(&W { x: Any { n: 2 } }))?;              // expect: 3
    out.line(&last_index([1u64, 2, 3].iter().copied()))?; // expect: 2
    Ok(())
}
```

## Why it's sound

- **Lemma A1 (decided where it's asked).** `project` works out each atom
  of a binding that hangs on a bound `B`, at the impl's arguments `σ`:
  out where `B[σ]` is proved from the bounds in scope, which holds in
  every copy made there (lemma B2 of docs/92-where-a-bound-holds.md); in
  where it's refuted, no impl that could prove it of any instance of
  `B[σ]`'s type being there to find (lemma N1 of
  docs/73-for-over-collections.md), which of a type with no parameter in
  it is that none proves it, as all its impls are loaded (the orphan
  rule). `refuted` gives up at a depth, saying it may hold: then it's
  left undecided, which allows less.
- **Lemma A2 (itself, and at most its binding).** Where it isn't decided,
  `<X as Tr>::e` stays itself. The impl `project` found for `X` is the
  impl in every copy (coherence, and `alone` of docs/73), so in each it's
  that binding with those of the atoms whose bounds don't hold there:
  within the binding with every atom (`upper`). So in a set within
  another, it may be replaced by that, as a bound's associated effect is
  by its bound (lemma E2 of docs/68.1-effect-bindings.md); in the bigger
  set, never, and equal it's only to itself.
- **Lemma A3 (inside the impl).** The impl's own associated types are
  equalities in its methods (`ImplDef::local_eqs`), but not an associated
  effect that hangs on a bound: inside, where the bound may hold of some
  copies and not others, it stays itself, as anywhere else it isn't
  decided. As an equality with its binding, `with Self::e` would allow a
  body to panic in a copy whose `Self::e` has none.
- **Lemma A4 (what says it).** A set a function says is taken apart
  (`guarded_set`): each projection that stays itself for a bound into its
  binding, and each atom with the bounds of all the places it's from, a
  binding's or the function's own `unless`. In a copy, an atom is in the
  set unless every one of its bounds holds there. The body is walked once
  allowing all of it, and once under all the bounds of each atom
  together, where no site may have that atom: each site's effects worked
  out again there, and asked as the walk without the bounds asks
  (`missing_effects`). By lemma B1 of docs/92-where-a-bound-holds.md,
  every copy's body has only what the set is in that copy; and a caller
  works the set out at its own types, which is what the copy has.
- **Lemma A5 (an impl's method).** An impl's method has at most its
  trait's method's effects, as each copy has them: both are taken apart;
  an atom the trait's has only unless bounds hold, the method may have
  only unless some of the same bounds do; one the trait's has always, it
  may have either way; its other items are among the trait's. So in each
  copy, what a call through the trait counts on holds of the body that
  runs.
- **Lemma A6 (copies).** Monomorphisation decides every projection by the
  same `project`, every type being known (A1), and leaves out of each
  copy of a method each atom it says `unless` a bound that holds of the
  copy's types (lemma B8 of docs/92). The interpreter checks every
  function on the stack at each effect, as before.
- **Lemma A7 (the stack).** Lemma B7 of docs/92, with A4 and A5 for what
  each function says: a copy whose set has no `panic` has a body with no
  site of it, by its walk under the bounds that hold there.

## What it doesn't do

- **A set within such a projection** has to be within it as written. A
  closure that panics only where `T` isn't `Low` can't be given where a
  `fn() -> u8 with <W<T> as Go>::e` is needed: what a closure's body has
  is worked out once, without the bound.
- **A bound of `Self`**, or of any type only partly a parameter, in an
  impl's associated effect: `Self: Low` of `W<T>` hangs on what `T` is,
  which a bound of `T`'s says.
- **A trait's default that says `unless`, in an impl's method** still
  can't have the effect where the impl's type may meet the bound, rather
  than being walked under it (docs/92-where-a-bound-holds.md).
- **`chars` isn't bounded yet**, so `s.chars().enumerate()` may still
  panic.

## Decisions

- **D1: the impl's method says it either way**, `with Self::e` or spelled
  out with `unless`, checked as each copy has it. Only spelled out would
  have refused `with Self::e`, which impls write today; only `with
  Self::e` would hide where the method panics.
- **D2: what a function says is checked as it is in each copy, now.** A
  projection that stays itself in a `with` is taken apart into its
  binding, and the body walked under its bounds, rather than matched as
  written, which would refuse `with Self::e` in the impl.
- **D3: undecided stays itself**, rather than being its binding with every
  atom everywhere, which would make `enumerate().map(..)` panic again in
  generic code, or deciding by what's known, which in a bigger set would
  say `{panic}` is within `<Enumerate<I> as Iterator>::e` where `I`
  may be bounded.

## Exercises

1. In `last_index` above, say `with {I::e, panic}` instead. Which walk no
   longer happens, and what does a caller of a slice's iterator lose?
2. Write `fn pairs<I: Iterator>(it: I) -> usize` that counts
   `it.enumerate().enumerate()`, saying `panic unless I: BoundedIterator`.
   How many bounds is its body walked under, and which?
3. Why is `{panic} ⊆ <Enumerate<I> as Iterator>::e` refused in code generic
   in an `I: Iterator`, though `<Enumerate<I> as Iterator>::e ⊆ {I::e,
   panic}` holds there?
4. Give `ImplDef::local_eqs` back the bindings that hang on bounds. Which
   test fails, and does the interpreter notice?
5. In `traits::decided`, call a bound that isn't proved refuted. Which
   tests fail?

## Reading

- Eisenberg, Vytiniotis, Peyton Jones and Weirich, *Closed type families
  with overlapping equations* (2014): a type family application that
  may still match an equation once more is known stays itself, "stuck",
  as an undecided projection does here.
- The Rust compiler's dev guide, *Normalization*: rigid projections, and
  why one that can't be normalised yet must be equal only to itself.
- Leijen, *Koka: programming with row polymorphic effect types* (2014):
  effect sets with variables, compared by inclusion, where an unknown
  part is never assumed empty.
