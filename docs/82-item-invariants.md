# Step 82: what an iterator gives

Code: `src/check/refine/items.rs` (`Walk::envelope`, `Walk::make_items`,
`Walk::given_items`, `Walk::adapter_items`, `Walk::item_params`,
`Walk::bound_states`); the protocol call, its frame and lends in
`Walk::mir_call` and `Walk::lent_keeps` (`src/check/refine/mir.rs`);
`Cand::Gives` in `src/check/refine/targs.rs`; `Range`'s `next` and
`next_back` in `core/ops.t`.

A `for` over a range knew what it gave: `for r in 0..8` gave each `r`
below 8, as `Range`'s `next` says it gives a value below its end, which it
keeps (docs/53). Through an adapter, it didn't. `Rev`'s `next` says it
gives an `Option<I::Item>`, which says nothing, and so did every adapter's
but for a list's elements, whose refinement is in their type (docs/63).
Retrospective 12 queued it, and t-chess had worked around it since 0.78.0,
printing rank `7 - k` for `k` in `0..8`:

```text
fn need(_r: { r: usize | r < 8 }) {}

fn f() {
    for r in (0..8).rev() {
        need(r); // error, on 0.96: can't prove `r < 8`
    }
    for h in l.iter().map(|v| half(v)) {
        // error, on 0.96: what `half` gives is forgotten
    }
    for r in 3..8 {
        let _ = r - 3; // error, on 0.96: `r - 3` may overflow
    }
}
```

## What every value an iterator gives has

An iterator's *protocol* is the methods of `Iterator` and
`DoubleEndedIterator` that take it by `&mut`: `next`, `next_back`, `any`
and `all`. What the checker now keeps of an iterator is its *items*: what
every value its protocol gives has, now or after any number of calls of
it. A call of `next` gives one of those, and what it leaves behind gives
only those too, as what it gives later is what the first would give
later (lemma I1). Through any adapter of `core`, a range gives what it
gives on its own:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    for r in (5..8).rev() {
        out.line(&a[r])?;
    }
    // expect: 17
    // expect: 16
    // expect: 15
    for (i, r) in (6..8).enumerate() {
        out.line(&f"{i} {a[r]}")?;
    }
    // expect: 0 16
    // expect: 1 17
    for (r, q) in (0..8).zip((0..8).rev()).skip(7) {
        out.line(&f"{a[r]} {a[q]}")?;
    }
    // expect: 17 10
    let b: [u8; 3] = [1, 2, 3];
    for r in 5..8 {
        out.line(&b[r - 5])?;
    }
    // expect: 1
    // expect: 2
    // expect: 3
    let mut it = 2..5;
    while let Some(r) = it.next_back() {
        out.line(&b[r - 2])?;
    }
    // expect: 3
    // expect: 2
    // expect: 1
    Ok(())
}
```

An object gets its items two ways: from its own type's signatures, and
from the iterators and closures it was made of.

## From an iterator's own signatures

`Range`'s `next` now says it gives a value at least where it started and
below its end, and that its start never goes back; `next_back`, a value
at least its start and below the end it had, and that its end only comes
down:

```text
fn next(&mut self) -> Option<{ v: T | v >= old(self.start()) && v < self.end() }>
ensures self.end() == old(self.end()), self.start() >= old(self.start())

fn next_back(&mut self) -> Option<{ v: T | v >= self.start() && v < old(self.end()) }>
ensures self.start() == old(self.start()), self.end() <= old(self.end())
```

The checker reads those of any iterator whose impls define `next` and
`next_back` alone (`Walk::envelope`). Each measure every one of them keeps
or moves one way, which it proves of their postconditions as a loop's
frame does (docs/52), bounds every state the iterator can get to from
this one: `start` never below what it is now, `end` never above. And what
each says of the value it gives, at a state within those bounds, holds of
every value it gives: of `a..b`, at least `a` and below `b`. An iterator
of the program's own gets the same:

```rust
/// Counts up from `at` to below `hi`.
#[derive(Discard)]
struct Up { at: u32, hi: u32 }

impl Up {
    fn new(at: u32, hi: u32) -> { u: Up | u.at() == at && u.hi() == hi } { Up { at, hi } }

    #[measure]
    fn at(&self) -> u32 { self.at }

    #[measure]
    fn hi(&self) -> u32 { self.hi }
}

impl Iterator for Up {
    type Item = u32;
    effect e = {};

    fn next(&mut self) -> Option<{ v: u32 | v >= old(self.at()) && v < old(self.hi()) }>
    ensures
        self.hi() == old(self.hi()),
        self.at() >= old(self.at()),
    {
        if self.at < self.hi {
            let v = self.at;
            self.at = self.at + 1;
            Some(v)
        } else {
            None
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 6] = [10, 11, 12, 13, 14, 15];
    for v in Up::new(3, 6) {
        out.line(&a[v as usize])?;
    }
    // expect: 13
    // expect: 14
    // expect: 15
    Ok(())
}
```

A measure that may move either way bounds nothing, and then neither does
what's said of it: if `next` said `ensures self.hi() >= old(self.hi())`,
`v < old(self.hi())` would say nothing of the second value
(`tests/t/err_soundness_items_measure_grows.t`).

An iterator generic in what it gives, as a slice's `Iter<'a, T>` is, its
`T` free (markers its only bounds), gives only values of `T` it holds
(lemma A2 of docs/60): so what's known of those is what it gives. A
list's refined elements went through adapters by their type already;
now a closure `map` is given knows them too.

## From what an adapter is made of

`Rev<I>`'s impl is generic in `I`. It can't make a value of `I::Item`:
nothing it can call gives one but `I`'s own protocol. So every value it
gives is one its `I` gave, and what the `I`'s items are is what its are
(lemma I4). The same holds of `Skip`, `Filter`, `Enumerate`'s second part,
each part of `Zip`'s, and of a program's own adapter of that shape:

```rust
/// Every other value of `iter`.
#[derive(Discard where I: Discard)]
struct Halve<I> { iter: I }

impl<I: Iterator> Iterator for Halve<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Some(_) => self.iter.next(),
            None => None,
        }
    }
}

fn halve<I: Iterator>(iter: I) -> Halve<I> where I::Item: Discard { Halve { iter } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 6] = [10, 11, 12, 13, 14, 15];
    for v in halve((0..6).rev()) {
        out.line(&a[v])?;
    }
    // expect: 14
    // expect: 12
    // expect: 10
    Ok(())
}
```

Which `I` an adapter holds is the one it was made with: `rev`, `halve`
and every function generic in an iterator `X`, its bounds the protocol's
traits and markers, can't make an `X` either, so the one it gives back is
one it was given, or what that one became by its protocol (lemma I3). Of
a function given two, either (`err_soundness_items_maker_either.t`); one
lent `&mut`, after the call, what it was.

`Map<I, F>` gives what `F` gives, which is what the closure given to
`map` gives: what its body knew where it returned, as a call it's given
to straight counts on (docs/74), or a function's refined result. And the
closure is called only with what `I` gives, so it's walked knowing that,
as `Option::map`'s closure knows what's in the `Some`:

```rust
fn half(v: u32) -> { r: u32 | r <= 50 } { v % 100 / 2 }

fn need50(r: { r: u32 | r <= 50 }) -> u32 { r }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [1, 2, 3, 4];
    for h in (98..101).map(half) {
        out.line(&need50(h))?;
    }
    // expect: 49
    // expect: 49
    // expect: 0
    for v in (0..4).rev().map(|x| a[x]).skip(2) {
        out.line(&v)?;
    }
    // expect: 2
    // expect: 1
    Ok(())
}
```

What would let an adapter give a value its iterator didn't, the checker
refuses to take as one:

- a field that could hold an item, `first: Option<I::Item>`
  (`err_soundness_items_stored.t`);
- a closure that gives items, `F: FnMut() -> I::Item`
  (`err_soundness_items_closure_source.t`);
- a bound that says what the items are, `I: Iterator<Item = usize>`, which
  lets it make them (`err_soundness_items_eq_bound.t`), or any bound on a
  type with `I` in it but `I`'s own (`err_soundness_items_other_bound.t`);
- its iterator behind a `&mut`, or in a `Cell`, where another could be put
  without its protocol (`err_soundness_items_by_ref.t`).

And an iterator whose impl overrides a default method could give anything
through it, `any` its closure, so its own signatures say nothing
(`err_soundness_items_override.t`).

## What keeps it

A call of the protocol keeps what an iterator gives. Anything else that
changes it, an assignment, a call it's lent to that isn't generic in it,
a write through a reference to it, is another iterator, which gives what
it gives (`err_soundness_items_assign.t`, `_lend.t`, `_ref_write.t`). A
loop keeps it as its frame keeps a measure (docs/52): if every change to
the iterator in the loop is a call of its protocol, what it gave at the
start it gives at the head every time round (lemma I5).

At a call of a range's `next` itself, the states before and after are
known: they're within the bounds (`Walk::bound_states`), and the call's
own signature says the rest, with no new variables, so a loop over a
range costs what it did. Of a range that's one of several, as a function
given two may give back either, within one's bounds: not both
(`err_soundness_items_pick.t`). Through an adapter, the inner range's states
aren't known: what's said is that some two states within the bounds gave
it, new for each value, as two values are given from two states
(`err_soundness_items_witness.t`).

## Why it's sound

- **I1 (what keeps it).** An iterator's items hold of every value any
  sequence of its protocol's calls gives from it. A call of its protocol
  leaves a state whose sequences are the rest of the first's: so they
  hold of it too.
- **I2 (its own signatures).** Of an iterator whose impls define only
  `next` and `next_back`, the default methods call only those, by
  parametricity of their bodies in `Self`. Each of the two keeps each
  measure in its bounds as its `ensures` says, proved where it's checked;
  so, by induction on the calls, every state got to has its measures
  within the bounds of the first. Each value is given by one of the two,
  from such a state to such a state, with what its refinement says: so
  some two states within the bounds have it. New states for each value,
  as each is given from a state of its own.
- **I3 (conservation).** A function generic in `X`, whose bounds on `X`
  are `Iterator`, `DoubleEndedIterator`, `IntoIterator` and markers, on
  `X::Item` markers, and on no other type with `X` in it, with no equality
  bound giving an `X` (but the blanket impl's own `X::IntoIter = X`; not
  `X::Item = X`, `err_soundness_items_nested.t`), and an `X` whose impls
  are the defaults, can't make an `X`: no method it can call gives one but
  by wrapping one it consumes, or, `into_iter`, giving it back. So each it
  gives back, or leaves behind a `&mut`, is one it was given, or what one
  became by the protocol: whose items, by I1, are that one's.
- **I4 (adapters).** In an adapter's impl, generic in iterators `P` and
  closures `F: FnMut(..) -> U`, `P::Item` is abstract: no equality bound
  says what it is, its bounds are markers, no closure gives one, no field
  holds one. So each value of it the impl gives is one `P`'s protocol gave
  it, of a `P` it holds: one it was made with (I3), held where only it can
  change it (its fields' variance), so changed only by its protocol (I1).
  And each `U` is one its `F` gave, the one it was made with, whose
  summary holds of every run while it lives (lemmas C1, C2 of docs/74).
- **I5 (loops).** As F1 of docs/52: each change since the loop started
  was a call of the protocol, and the borrow checker lets nothing else
  change the iterator while it's lent.
- **I6 (use).** A call of `next` or `next_back` gives a value the protocol
  gives: by definition, the items hold of it.

The interpreter checks every refinement a value is given to, so a
wrong item would stop a test there.

## What it doesn't do

- `filter`'s closure said `true` of each value it gives, but that's in its
  body, not its declaration: `(0..8).filter(|x| *x > 2)` gives values below
  8, not above 2 (`err_items_filter_predicate.t`).
- `Enumerate`'s index is below no bound: what it counts and what its
  iterator has left aren't related.
- `for x in &mut it` goes through the `&mut`'s impl, which holds its
  iterator behind a reference: nothing's known of what it gives.
- Where two paths meet, an iterator that's another object on each forgets
  its items, until a loop's head, where its frame keeps them.
- A function returning `impl Iterator<Item = T>` says only `T`.

## Decisions

- **D1 approach**: what every value an iterator gives has, from its own
  signatures and, by parametricity, from what an adapter is made of; no
  syntax (recommended, chosen), rather than predicates declared on
  `Iterator` each impl defines (abstract refinements), or walking core's
  adapters' bodies at each type they're used at, which would make those
  bodies part of every program's proof.
- **D2 `Range`**: both ends, so `a..b` gives values at least `a` too
  (recommended, chosen), rather than only what `rev` needed.
- **D3 which iterators**: any whose impls define only `next` and
  `next_back`, from their signatures (recommended, chosen), rather than
  `Range` alone.

## Exercises

1. Write `struct Down { at: u32 }` counting down to 0, with `next`'s
   refinement and `ensures`, so that `for v in Down::new(5)` knows `v < 5`.
   Which way does its measure move?
2. Give `Halve` a field `last: Option<I::Item>` it never reads. Which
   program above is now refused, and why must it be?
3. Why does `Map<I, F>` need its `F` held by value? Write a program that
   would go wrong if an adapter held its closure in a `Cell<F>`.
4. `filter`'s closure is walked knowing what its iterator gives. Why is
   that sound, when what it says of each isn't kept?
5. Take `new for each value` out of I2: which test fails, and what would
   it accept?

## Reading

- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983): the
  abstraction theorem, I3 and I4's ground.
- Wadler, *Theorems for free!* (1989): what a polymorphic function's type
  says of it, with type classes as the functions they're given.
- Vazou, Rondon and Jhala, *Abstract Refinement Types* (2013): refinements
  that are parameters, what `Iterator` could have declared instead (D1).
- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust* (2023):
  refined Rust, and its associated refinements for traits.
- Floyd, *Assigning Meanings to Programs* (1967): invariants, here of a
  protocol, kept by each step.
