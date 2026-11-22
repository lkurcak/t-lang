# Step 88: iterator laws

Code: `Checker::law` (`src/check/traits/collect.rs`), with `has_glue`
(`src/traits.rs`), and `Checker::same_law_signature`
(`src/check/traits/impls.rs`); in `src/check/refine/laws.rs`,
`Checker::law_tables`, which pairs impls, and
`Checker::keep_laws`, which walks their methods again; `Walk::with_laws`
in `src/check/refine.rs`, where calls learn them; `Mono::laws_of` and the
law checks in `Interp::call`; `ExactSizeIterator` in `core/iter.t`.

An iterator over a slice knew how many values it had left: `len`, a
measure, which each `next` took one off. But each said so of its own
`next`, so only code that knew it had a slice's iterator could count on
it. Code generic in an iterator knew nothing of how many were left, and
`map`, `rev` and the other adapters had no `len` at all: a measure of
theirs would be of their iterator's, which a type parameter has none of.

What's wanted is a trait that has `len`, and says how `Iterator`'s `next`
changes it. A trait could say what its own methods promise (step 86), but
`next` is `Iterator`'s. Now a trait can state a *law*: what a method of
one of its supertraits promises of every type implementing it.

```rust
trait Counted: Iterator {
    #[measure]
    fn left(&self) -> usize;

    law fn next(&mut self) -> { r: Option<Self::Item> | r.is_some() == (old(self.left()) > 0) }
    with Self::e
    ensures
        self.left() + 1 == old(self.left()) || (self.left() == 0 && old(self.left()) == 0);
}

#[derive(Discard)]
struct Down { n: usize }

impl Iterator for Down {
    type Item = usize;
    effect e = {};

    // Says nothing more of itself: the law is checked of this body, as
    // `Down` implements `Counted`.
    fn next(&mut self) -> Option<usize> {
        if self.n > 0 {
            self.n = self.n - 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl Counted for Down {
    #[measure]
    fn left(&self) -> usize { self.n }
}

// Counts what it's given: as many as `left` said.
fn drain<I: Counted>(it: &mut I) -> { n: usize | n == old(it.left()) }
with I::e
where
    I::Item: Discard,
{
    let start = it.left();
    let mut n: usize = 0;
    while let Some(_x) = it.next() invariant n + it.left() == start {
        n = n + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&drain(&mut Down { n: 3 }))?; // expect: 3
    Ok(())
}
```

## What a law is

A law is written in a trait as the method it's of is declared, after
`law`, with no body: the same parameters, result, effects and bounds, and
what it promises, a refined result and `ensures`. It can't refine a
parameter: what a call through the method's own trait proves is all the
method's body may assume, whatever else its type implements.

- **Of a supertrait's method**: `next` above is `Iterator`'s, and
  `Counted: Iterator`. Every type implementing `Counted` implements
  `Iterator`, and keeps the law.
- **Of another trait's method**, written with `where Self: P`: a
  *conditional* law, kept by every type implementing both. That's how
  `ExactSizeIterator` says `next_back` takes one off too, of a
  double-ended iterator, which not every exact one is.
- **Of a method with a default body**: the default keeps it, for every
  type implementing the law's trait, as it's checked once, generically.
  An impl that writes its own body for the method is checked as any
  impl's is.
- **Not of a method of a trait with glue**: `Eq`, `Ord`, `Debug`,
  `Discard` and the other traits some of whose impls the compiler makes,
  by `#[derive]` or its own rules, as for the integers and tuples. Their
  methods there are glue, with no body to check a law against (H1 of
  retrospective 16, below).

What a law says is learned at every call of the method whose type
implements the law's trait where the call is: through a bound, `I:
ExactSizeIterator`, or of a type known to, as `Down` is to implement
`Counted`. A call through `I: Iterator` alone learns nothing of it.

```rust,error
fn more<I: Iterator>(it: &mut I) -> bool with I::e where I::Item: Discard { it.next().is_some() }

fn first<I: ExactSizeIterator>(it: &mut I) -> { b: bool | b } with I::e where I::Item: Discard {
    let k = it.len();
    let r = more(it);
    r == (k > 0)
}

fn main() {}
// error: can't prove `b` for the result of `first`
```

## Where a law is kept

For each impl of the law's trait, and each impl of the method's trait for
the same types, the method's body there is walked again, with both
impls' bounds in scope, to keep what the law promises. `Map<I, F>`'s
`next` is walked knowing `I: ExactSizeIterator`, which its impl of
`ExactSizeIterator` needs: so its call of `self.iter.next()` learns `I`'s
law, and `Map`'s `len`, `self.iter.len()`, comes out one less too.

An impl that doesn't keep it is refused where it's written:

```rust,error
#[derive(Discard)]
struct Stuck { n: usize }

impl Iterator for Stuck {
    type Item = usize;
    effect e = {};

    // Gives a value, but `len` stays.
    fn next(&mut self) -> Option<usize> {
        if self.n > 0 { Some(self.n) } else { None }
    }
}

impl BoundedIterator for Stuck {
    #[measure]
    fn most(&self) -> usize { self.n }
}

impl ExactSizeIterator for Stuck {}

fn main() {}
// error: can't prove `!r.is_some() || self.most() < old(self.most())` for the result of `next`
// error: can't prove `self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0)` for what `next` ensures
```

(Since step 92 an exact iterator's `len` is its `most`, a measure of
`BoundedIterator`, the trait `ExactSizeIterator` is under, which has a law
of its own: so there are two impls here, and two laws broken.
See docs/92-where-a-bound-holds.md.)

Two impls are paired only if what they're for is the same up to the
names of their type parameters, and the method's body is this package's,
so that it's walked here. Otherwise the law can't be checked, and that's
an error, not a promise taken on trust: an impl of `ExactSizeIterator`
for `Once<u8>` alone, where `Iterator`'s is for every `Once<T>`, would need
that body walked at `T = u8`, which isn't done yet.

What a law promises of a parameter is of its value as passed, so the
method it's of can't make that parameter `mut`, as an impl's method can't
for what its trait's promises (H2 of retrospective 15).

And a law can't be of a method of a trait whose impls aren't all written:
a derived `Eq`'s `eq`, or an integer's or a tuple's, is glue
monomorphisation makes, and nothing is walked to keep it. Until 0.102.1
such a law was learned and never checked, so this was accepted, and read
`arr[7]`:

```rust,error
trait Never: Eq {
    law fn eq(&self, other: &Self) -> { b: bool | !b };
}

#[derive(Eq, Discard)]
struct X { a: u8 }

impl Never for X {}

fn same<T: Never>(a: &T, b: &T) -> { r: bool | !r } { a.eq(b) }

fn main() with {} {
    let x = X { a: 1 };
    let arr = [0u8; 1];
    let i: usize = if same(&x, &x) { 7 } else { 0 };
    let _ = arr[i];
}
// error: a law can't be of `Eq`'s `eq`: some impls of `Eq` are the compiler's, derived or built in, with no body to check what it says of
```

## `ExactSizeIterator`

`core::iter::ExactSizeIterator`, in the prelude, has `len`, a measure,
and these laws. (Since step 92 `len` is `most` of `BoundedIterator`, the
trait it's under, where the laws of `map`, `enumerate` and `rev` are now
said, of `most`: docs/92-where-a-bound-holds.md.)

```text
law fn next(&mut self) -> { r: Option<Self::Item> | r.is_some() == (old(self.len()) > 0) }
with Self::e
ensures self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0);

law fn next_back(..) -> .. where Self: DoubleEndedIterator ensures ..;  // the same

law fn map<U, effect fx, F: FnMut(Self::Item) -> U with fx>(self, f: F)
    -> { m: Map<Self, F> | m.len() == self.len() };
law fn enumerate(self) -> { e: Enumerate<Self> | e.len() == self.len() };
law fn rev(self) -> { r: Rev<Self> | r.len() == self.len() } where Self: DoubleEndedIterator;
```

A slice's iterators, `Iter`, `IterMut` and `Copied`, an array's and a
list's `IntoIter` implement it, and no longer say it of their own `next`.
So do `Map`, `Enumerate` and `Zip` of exact iterators, and `Rev` of a
double-ended exact one. `Zip`'s is the shorter one's:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = [1u64, 2, 3, 4];
    let b = [10u64, 20, 30, 40];
    let m = a.iter().map(|x: &u64| *x * 2);
    let k: { k: usize | k == 4 } = m.len();
    out.line(&k)?; // expect: 4
    for x in it: a.iter().rev().map(|x: &u64| *x) invariant it.len() <= 4 {
        // `it` has one fewer than before `x` was given.
        out.line(&(x + b[3 - it.len()]))?;
    }
    // expect: 14
    // expect: 23
    // expect: 32
    // expect: 41
    out.line(&a.iter().zip(b.iter()).len())?; // expect: 4
    Ok(())
}
```

## `if` in a measure's body

A measure's body may say `if c { a } else { b }`, each a term of the
logic, as `Zip`'s `len` does: a function of the three. In the logic it's
a new variable that's `a` where `c` holds and `b` where it doesn't, as an
integer `match` of which variant a value is already was
(docs/59-lends.md).

```rust
#[derive(Discard)]
struct Two { a: usize, b: usize }

impl Two {
    #[measure]
    fn least(&self) -> usize {
        if self.a <= self.b { self.a } else { self.b }
    }
}

fn pick(t: &Two) -> { n: usize | n <= t.a && n <= t.b } { t.least() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pick(&Two { a: 4, b: 9 }))?; // expect: 4
    Ok(())
}
```

## Why it's sound

- **L1 (a law at a call).** A call of method `m` of trait `P`, whose
  `Self` is `X` where `X: S` holds, learns `S`'s law of `m` at `X`.
  Monomorphisation runs `X`'s impl of `P`'s `m` (static dispatch; one impl
  by coherence): its own method, or `P`'s default body, as `P` has no
  glue, the bodies the compiler makes for derived and built-in impls,
  which a law of its method would be refused for (`has_glue`, and
  `Mono::target` asserts it makes glue for no other trait). `X: S` holds by an impl of `S` for some `H` that `X` is,
  whose bounds hold at `X`; the impl of `P` whose `m` runs is for `H` too,
  up to the names of their parameters (else the pair was refused), so its
  body was walked with both impls' bounds, assuming what `m` asks, to keep
  what the law promises. A walk of a generic body is sound for every
  instantiation its bounds hold of, and they hold at `X`: so what the law
  promises holds when that body returns at `X`. With a default body,
  checked once with `Self: S`, the same, at `Self = X`.
- **L2 (what the walk assumes).** The second walk learns laws too: of its
  iterator's `next`, say, which `I: ExactSizeIterator` gives it, and of its
  own type's, if the body calls itself. Each is of a call nearer the
  leaves of the run, as a recursive function assumes its own contract:
  partial correctness, by induction on how deep the calls go.
- **L3 (conditional laws).** One kept where `X: S` and `X: P` both hold:
  by an impl of each whose types `X` is, so whose types unify. Every such
  pair is checked, in the package that has the later of the two, which
  sees both; a pair whose unifier isn't a renaming is refused.
- **L4 (measures).** What a law says of `len` is of the measure at `X`,
  `X`'s impl's, resolved by type in the walk and in monomorphisation
  alike (B2 of docs/86-trait-contracts.md); in generic code it's
  unknown but for what's been learned of it.
- **L5 (modularity).** An impl's obligations are its own package's: the
  method's body it pairs with must be there, or it's refused. Adding a
  package adds only its own pairs, so adding a dependency never makes a
  program that checked stop checking.
- **Sealed adapters (lemma S of docs/83-sources.md), kept.** An adapter
  is sealed if nothing in its module takes one but its protocol's
  methods; its `len` takes one too, but a measure has no effects and gives
  an integer or a `bool`, so it can neither hand out what the adapter
  holds nor change it: measures don't unseal a type.
- **L6 (the oracle).** The interpreter checks, at each call of an impl's
  method or of a copy of a trait's default body, each law of the method
  whose trait the copy's type implements, as it does the method's
  trait's contract. (Until 0.102.1, not of a default body's copy.)

## What it doesn't do

- **`count` and `enumerate` still say `with panic`.** (Until step 92, of
  `count`: docs/92-where-a-bound-holds.md. `enumerate`'s index has had
  its invariant since step 93, docs/93-invariants-where.md, and its
  effect waits for step 94.)
  An exact iterator's
  count can't overflow, but an effect is the method's, the same for every
  iterator: it can't depend on whether `Self` is exact. Effects that do
  are a design of their own. Meanwhile `len()` is an exact iterator's
  count, and `for x in it: e invariant i + it.len() == n` its index.
- **`Skip` and `Filter` have no `len`.** (Until step 92, where a clause
  of an invariant can say where it holds.) Their `next` loops, and an
  invariant naming `len` can't be written in a body shared with iterators
  that have none. `Filter` could only ever have an upper bound.
- **`Range` has no `len`.** Its `Iterator` impl is for every `Range<T>`
  of an integer type, and a `len` that's a `usize` is only of some of
  those: the law would need the body walked at `T = usize`.
- **`zip` has no law of its result's `len`**: its other iterator needn't
  be exact.
- **`&mut I` isn't exact**, as a measure can't read through a `&mut`
  (docs/86, H1): `for x in &mut it` doesn't know how many are left.
- A law can be of a method with a default body only in that method's
  trait's package, where the body is checked.
- **No law of `Eq`'s `eq`, `Ord`'s `cmp` or `Debug`'s `fmt`.** Some of
  their impls are glue. One kept by written impls only could be checked,
  but a type's derive would then refuse it; a derive that wrote bodies
  the checker walks would let laws of them be.

## Decisions

- **D1: `law fn`**, written as the method is declared, with what it
  promises; one of another trait's method says `where Self: P`. A path,
  `fn Iterator::next`, was the alternative: shorter, but it reads as
  declaring a method.
- **D2: conditional laws** for `next_back`, rather than a third trait
  every exact double-ended iterator would implement.
- **D3: `count` and `enumerate` keep `panic`.** Stopping when the index
  runs out, or saturating the count, would be wrong without a word on a
  32-bit target, where 2³² values can be gone through.
- **D4: `if` in a measure's body**, for `Zip`'s `len`.
- **D5 (step 89, retrospective 16): no law of a method of a trait with
  glue**, refused where the law is. Refusing each impl of the law's
  trait at a type whose method could be glue keeps laws of hand-written
  `Eq` impls, but needs a complete account of which types have which
  built-in impls, tuples under conditional laws included, for a law no
  derived type could keep; a derive writing bodies to check is larger.
  Either can come later, as it accepts more.

## Exercises

1. Write an iterator over the values `a, a + 1, .., b - 1` of a struct
   with fields `a` and `b`, and make it an `ExactSizeIterator`. What must
   `len` be when `a > b`?
2. Why can't a law refine a parameter? Write a program that would go
   wrong if one could.
3. `Pairs<I>`, whose `next` takes two of `I`'s values for each it gives,
   can't say its `len` is `I`'s. What could it say, and what would its
   `next` need to know of `I` to keep it?
4. A conditional law is checked for each pair of impls. Why must the
   types of the two impls be the same up to their parameters' names?
   Write two impls where they aren't, and say which types the law would
   be unchecked for.
5. Why does `Enumerate<I>`'s `next` keep `next`'s law, though it may
   panic?
6. A law of `Eq`'s `eq` is refused even in a program whose every `Eq`
   impl is written. Which impls would it have been learned of anyway?

## Reading

- Liskov and Wing, *A Behavioral Notion of Subtyping* (1994): a subtype's
  methods meet the supertype's specifications, and may say more.
- Leavens and Naumann, *Behavioral Subtyping, Specification Inheritance,
  and Modular Reasoning* (2015), and JML's `also`: a subtype restating a
  method with more specification.
- Wadler and Blott, *How to Make Ad-hoc Polymorphism Less Ad Hoc* (1989),
  type classes, whose laws were left to the programmer; Vazou, Seidel,
  Jhala et al., *Refinement Types for Haskell* (2014), and Liquid
  Haskell's refined type classes, which check them.
- Rust's `ExactSizeIterator`, whose `len` is a promise nothing checks,
  and `TrustedLen`, an `unsafe` trait for one unsafe code may count on.
