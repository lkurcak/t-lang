# Step 6b: associated types

Code: `src/traits.rs` (`Projections`: working projections out),
`src/check/traits.rs` (associated types in traits and impls, the rules for
them, and resolving projections), `src/check/unify.rs` (projections on types
not known yet), `src/borrowck/regions.rs` and `src/borrowck/build.rs`
(lifetimes and projections) and `src/mono.rs`.

```rust
trait Container {
    type Item: Discard;

    fn first(&self) -> Self::Item;
}

#[derive(Discard)]
struct Ints { x: int, y: int }

impl Container for Ints {
    type Item = int;

    fn first(&self) -> int { self.x }
}

#[derive(Discard)]
struct Flags { on: bool }

impl Container for Flags {
    type Item = bool;

    fn first(&self) -> bool { self.on }
}

fn get<C: Container>(c: &C) -> C::Item { c.first() }

fn main() {
    print(get(&Ints { x: 1, y: 2 }));     // expect: 1
    print_bool(get(&Flags { on: true })); // expect: true
}
```

A trait can say that each impl picks a *type*, as well as methods: `type
Item;` in the trait, `type Item = int;` in an impl. Code names that type with
a *projection*, `<C as Container>::Item`, "the `Item` of `C`'s impl of
`Container`", or `C::Item` for short. `get` returns an `int` for `Ints` and a
`bool` for `Flags`: its result type depends on its type argument, through the
impls. `Container::Item` is a function from types to types, defined case by
case, an impl at a time. Haskell calls that an open *type family*.

## Projections

`<T as Tr>::Item` can be written anywhere a type can. The short `T::Item` is
for a type parameter, and `Self::Item` for the type of the trait or impl it's
in: their bounds say which trait it is. If two of `T`'s traits have an
`Item`, `T::Item` is ambiguous, and the long form says which. A supertrait's
associated type is its subtrait's too: with `trait Sized2: Container`, a `T:
Sized2` has a `T::Item`.

On any other type the trait has to be written, as in Rust. Unlike for a type
parameter, no bound says which trait it would be:

```rust,error
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = int;
}

fn main() {
    let x: bool::Out = 1;
}
// error: `bool::Out` is short for a projection on a type parameter; on `bool`, say which trait's `Out` it is: write `<bool as Tr>::Out`
```

`<bool as Tr>::Out` is `int`: once the type it's on is known, a projection
*normalises* to what its impl says. In generic code it often isn't known:
`C::Item` in `get` is a type of its own, *rigid*, equal only to itself, like
`C`. That's what keeps generic code honest: `get` can't treat its result as
an `int`, since for `Flags` it isn't one. Dictionary passing explains it:
compiling `get` once, its caller would pass a dictionary for `C: Container`
holding `first` and the type `Item`, and `get` could only use them as given,
like the abstract type of an ML module.

Default methods see `Self::Item` rigid too. That's why an associated type
can't have a default: a default method could be checked knowing `Item` is
`int`, and an impl that picks `bool` would make that method wrong.

## Bounds

Generic code knows nothing of `C::Item` but what bounds say. There are three
kinds:

```rust
trait Make {
    type Out;

    fn make(&self) -> Self::Out;
}

#[derive(Discard)]
struct Two { n: int }

impl Make for Two {
    type Out = int;

    fn make(&self) -> int { self.n * 2 }
}

// An equality bound: `M::Out` is `int`, so it can be added.
fn plus_one<M: Make<Out = int>>(m: &M) -> int { m.make() + 1 }

// A bound on the projection: `M::Out` may be thrown away.
fn ignore<M: Make<Out: Discard>>(m: &M) -> int {
    let _ = m.make();
    0
}

fn main() {
    print(plus_one(&Two { n: 3 })); // expect: 7
    print(ignore(&Two { n: 3 }));   // expect: 0
}
```

The third is in the trait, as in `type Item: Discard;` above: every impl's
`Item` must be `Discard`, which is checked at each impl, and generic code
may then assume it of any `C::Item`.

Linear types (docs/06a.1-linear-types.md) make these needed often: generic
code can't drop a `C::Item` unless one of them says it may.

```rust,error
trait Make {
    type Out;

    fn make(&self) -> Self::Out;
}

fn ignore<M: Make>(m: &M) -> int {
    let _x = m.make();
    0
}
// error: `_x` is never used up, and its type, `M::Out`, isn't `Discard`; to let it be thrown away, add a bound: `M: Make<Out: Discard>`
```

Should `Iterator` say `type Item: Discard;`? Then nothing could iterate over
values that must be used up, like files to close. The bound at the use, where
it's needed, doesn't force that choice on every user.

A bound on a projection can only be on a function's type parameter. On an
impl, `impl<T: Tr<Out: Show>> Show for W<T>`, proving `W<X>: Show` would
mean proving something of what `X::Out` stands for, which can be a bigger
type than `W<X>`, and resolution's termination argument (docs/06-traits.md)
is that each step is about a smaller type. Equality bounds don't need
proving by resolution, only working out, so they can be on impls and
structs too. For the same reason a derived impl of a struct with a field of
type `T::Out` can only rely on what the trait declares:

```rust,error
trait Tr {
    type Out;
}

#[derive(Discard)]
struct H<T: Tr> { f: T::Out }
// error: `H` can't be `Discard`: its field `f` holds a `T::Out`, and `Tr` doesn't say that's `Discard`; declare it as `type Out: Discard;`
```

## Lifetimes

This is the first time trait resolution decides *types*, so a wrong impl, or
a wrong lifetime, means reading one type as another. Since docs/06-traits.md,
finding an impl ignores lifetimes, which is sound because impl headers are
*parametric*: each lifetime once, never `'static`, so the impl found for
`Pair<'x, 'y>` was checked for every two lifetimes. But working out an
associated type must use them:

```rust,error
#[derive(Discard)]
struct Pair<'a, 'b> { a: &'a int, b: &'b int }

trait Second {
    type Out;

    fn second(self) -> Self::Out;
}

impl<'a, 'b> Second for Pair<'a, 'b> {
    type Out = &'b int;

    fn second(self) -> &'b int { self.b }
}

fn get<T: Second>(t: T) -> T::Out { t.second() }

fn main() {
    let long = 1;
    let r;
    {
        let short = 2;
        r = get(Pair { a: &long, b: &short });
    }
    print(*r);
}
// error: `short` doesn't live long enough
```

`<Pair<'x, 'y> as Second>::Out` is `&'y int`: matching the header
`Pair<'a, 'b>` to the type gives `'a` and `'b` the type's lifetimes, each in
its place, and the associated type is what the impl says with those. That
it's a function of the type, regions and all, is what makes it sound to work
out at different times: in generic code, in the borrow checker, in
monomorphisation. Normalising commutes with substitution: if `<S as Tr>::Out`
is `U`, then `<σS as Tr>::Out` is `σU`, as the impl that matches `S` matches
`σS` too, and no other can (coherence).

So the borrow checker works projections out itself, after giving each region
of a call's type arguments a variable (`Builder::instantiate`). Taking the
checker's answer, worked out with regions erased, and giving that new
regions, would be the same bug as 6a's `Pair<'a, 'a>`: a reference tied to
nothing, which the interpreter then catches reading freed memory.

Two rules keep "the impl's type with the header's lifetimes" well defined:

- **Constrained:** an associated type can only mention lifetimes and type
  parameters that are in the impl's type, outside every function type. Not in
  it at all, `impl<'a> Tr for S { type Out = &'a int; }`, nothing says what
  `'a` is, so each mention of `<S as Tr>::Out` could be a different type, and
  `fn put<T: Tr>(slot: &mut T::Out, v: T::Out)` could write a short-lived
  reference into a long-lived slot. Only inside a function type, it's
  decided as a lifetime only that function type means, since finding an impl
  doesn't look at what function types are generic over:

```rust,error
trait Tr {
    type Out;
}

impl<A, B> Tr for fn(A) -> B {
    type Out = B;
}
// error: `B` is only inside a function type in `fn(A) -> B`
```

  At `fn(&int) -> &int`, `B` would be the `&int` it returns, whose lifetime
  is whatever a call picks.

- **Closed:** every lifetime in an associated type is one of the impl's or
  `'static`, written out: `type Out = &int;` is an error. So a projection's
  regions are its type's and `'static`, and `T: 'r` gives `T::Out: 'r`
  (Rust's RFC 1214). That's how the borrow checker knows how long a rigid
  projection lives: as long as its type does, or as long as the signature
  says it does, but no longer.

A rigid projection is *invariant* in its type: `W<'a>` may be covariant, but
`<W<'a> as Tr>::Out` can be a `&'a mut &'a int`, so a struct with a field of
type `T::Out` is invariant in `T` (`AdtInfo`'s `vary`).
`tests/t/err_soundness_assoc_variance.t` shows what covariance would let
through.

## Inference

Working out a projection only goes forward, from the type to what it stands
for, never back: different types can have the same `Item`, so knowing the
`Item` doesn't say which. While the type isn't known, unifying the projection
with anything waits as a *goal* (`settle_goals`) until it is:

```rust,error
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = int;
}

fn mk<T: Tr>(x: T::Out) -> T::Out { x }

fn main() {
    print(mk(1));
}
// error: cannot infer `T` for this use of `mk`
```

There's only one impl, but which impls there are mustn't change what a call
means. That's the one exception to docs/06-traits.md's "obligations never
drive inference", and only forwards, where coherence makes it a function.

## Termination

Working out a projection can need others: `impl<T: Tr> Tr for W<T> { type
Out = Option<T::Out>; }`. Each projection in an associated type must be on a
type smaller than the impl's, built from its parts, so each step is about a
smaller type, as for resolution. A blanket impl, `impl<T: A> B for T`, can
work out `B`'s from `A`'s on the same `T`, as long as those steps between
traits don't go round in a cycle (`assoc_termination`): then each step is
smaller, or the same size and further down an order of the traits.

Two other things can go wrong only through impls, which is why they're
checked for the whole program:

- A type that holds itself: `struct S<T: Tr> { x: T::Out }` with `impl Tr for
  int { type Out = S<int>; }` has an infinite size, which neither says alone
  (`assoc_sizes`).
- Polymorphic recursion (docs/04-generics.md): `T::Out` can be as much bigger
  than `T` as an impl likes, so a use of `f::<T::Out>` in `f`, or a method
  call on a `T::Out` that may run an impl that leads back, is a cycle that
  could make monomorphising go on forever. Edges through a projection have
  unbounded weight (`generics::polymorphic_recursion`), and so do types that
  impls' associated types mention (`assoc_regular`). That rejects some
  programs that would stop, never one that doesn't.

## Design decisions worth noticing

- **Selection ignores lifetimes; projection uses them.** Which impl is the
  same at every lifetime, as headers are parametric; what it says isn't.
- **Every rule is the restriction its soundness argument needs.** Each could
  be relaxed later without breaking a program: constrained parameters only
  where a bound region would really escape; equality bounds with projections
  in them, with a termination check of their own; bounds on projections in
  impls, with a whole-program check that resolution can't cycle.
- **Forward-only inference.** A call's meaning never depends on which impls
  happen to exist.
- **No defaults** for associated types, which Rust has kept unstable for the
  reason above.
- **The shorthand is for type parameters only.** `S::Out` for a type would
  need to search the impls to say which trait, and a new impl elsewhere could
  change the answer.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| defaults for associated types | default methods would rely on them | — |
| generic associated types (`type Item<'a>`) | no higher-kinded types yet | — |
| equality bounds with projections in them | needs a termination check | — |
| bounds on projections in impls and structs | breaks resolution's termination argument | — |
| `Iterator` and `for` | needs closures as traits first | 6c |
| `dyn Trait<Item = int>` | no trait objects | — |

## Exercises

1. Write `impl<T: Tr> Tr for Box<T> { type Out = T::Out; }` and a function
   that uses it at `Box<Box<int>>`. Trace how `Projections::normalise` works
   out `<Box<Box<int>> as Tr>::Out`, and why it ends.
2. Make `regions_at` give every lifetime `Region::Erased`. Which tests
   fail, and how? Run `tests/t/err_soundness_assoc_pair.t` under the change
   and read what the interpreter says.
3. Change `vary` so a projection is covariant in its type. Show the program
   in `err_soundness_assoc_variance.t` freeing memory it then reads.
4. Relax the constrained rule: allow a parameter that's only inside a
   function type, but reject working out a projection when a region bound
   by a function type would end up in the result. Why is that still stable
   under substitution? Where would the check have to go so that it's never
   made after monomorphisation?
5. Allow `T::Out: Show` on impls. Write a program whose resolution then goes
   on forever, and say what whole-program check would rule it out.
6. Why does forward-only inference reject `mk(1)` above, though `bool` is
   the only `T` it could be? Add `impl Tr for () { type Out = int; }` and
   argue that a check that accepted it before would now have to change its
   mind.
7. Write `Iterator` with `type Item;` and `fn next(&mut self) ->
   Option<Self::Item>;`, a counter that implements it, and a `sum` with an
   equality bound. What would `for x in it { .. }` desugar to?

## Reading

- Manuel Chakravarty, Gabriele Keller, Simon Peyton Jones and Simon Marlow,
  *Associated Types with Class*, POPL 2005. Associated types in Haskell type
  classes.
- Manuel Chakravarty, Gabriele Keller and Simon Peyton Jones, *Associated
  Type Synonyms*, ICFP 2005. The type-level functions themselves.
- Tom Schrijvers, Simon Peyton Jones, Manuel Chakravarty and Martin
  Sulzmann, *Type Checking with Open Type Functions*, ICFP 2008. Normalising
  and the conditions for it to end.
- Richard Eisenberg, Dimitrios Vytiniotis, Simon Peyton Jones and Stephanie
  Weirich, *Closed Type Families with Overlapping Equations*, POPL 2014.
- Rust RFC 195, *Associated items*, and RFC 1214, *Projections, lifetimes
  and well-formedness*: how a projection's lifetimes are related to its
  type's.
- David MacQueen, *Modules for Standard ML*, 1984, and Xavier Leroy,
  *Manifest Types, Modules, and Separate Compilation*, POPL 1994: abstract
  types with equations, which rigid projections and equality bounds are a
  form of.
