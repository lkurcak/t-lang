# Step 29: closed and sealed traits

Code: `check/traits/termination.rs` (`closed_bounds`), `check/traits/dicts.rs`
(the graph against polymorphic recursion), `check.rs`
(`no_polymorphic_recursion`), `traits.rs` (`refuted`, `sealed_refutes`,
`CLOSED_TRAITS`), `check/traits/impls.rs` (`orphan`, `coherence`),
`load.rs` (no more chain rule), and `core/cmp.t`, `core/fmt.t`,
`core/num.t`.

The second retrospective found one design problem that mattered more than
the rest: three of the checker's rules were about the whole program. That
proving what implements what ends, that associated types are worked out in
finitely many steps, and that monomorphising ends, were each a graph over
every package's traits and impls. A package checked alone could be fine,
and two such packages, which don't know each other, could break a rule
together. So step 26 made the packages of a program form a chain, each
depending on the one before, and a program couldn't use two libraries that
don't know each other.

This step makes every rule a rule about one package and what it depends
on, and drops the chain. Two packages that check alone check together:
adding a dependency never breaks a program. Along the way it closes a hole
(a `discard` could make monomorphisation go on forever), and adds what the
same retrospective asked of the integer types: `sealed` traits, whose impls
are all there will ever be, so that one impl for every `T: Int` doesn't
overlap one for `bool`.

## Closed traits

Proving that a type implements a trait by an impl needs what the impl
needs. A bound on a type parameter that's part of the impl's type is about
a smaller type, and that ends (docs/06-traits.md). A bound on what isn't,
a projection like `I::Item` or a parameter another bound decides, can be
about a bigger type (docs/07.1-impl-parameters.md). Before, such a bound
could be of any trait that couldn't lead back to the impl's, along a graph
of every trait. Now it can only be of a *closed* trait: `Discard`, `Copy`,
`Fn`, `FnMut`, `FnOnce`, `Int`, `Signed` and `Unsigned`, whose proofs only
ever need more of them.

```rust,error
trait Show {
    fn show(&self) -> i64;
}

trait Total {
    fn total(&self) -> i64;
}

struct W<I> { iter: I }

impl<I: Iterator> Total for W<I> where I::Item: Show {
    fn total(&self) -> i64 { 0 }
}
// error: this impl's bound `I::Item: Show` isn't on a type parameter of `W<I>`
```

Of a closed trait it's fine, as core's `Filter` needs its items to be
`Discard`, and as this needs them to be integers:

```rust
trait Count {
    effect e;

    fn positives(&mut self) -> i64 with {Self::e, panic};
}

#[derive(Discard where I: Discard)]
struct Positives<I> { iter: I }

impl<I: Iterator> Count for Positives<I> where I::Item: Int {
    effect e = I::e;

    fn positives(&mut self) -> i64 with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + if x > 0 { 1 } else { 0 },
                Option::None => return n,
            }
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(Positives { iter: -3..4i32 }).positives())?; // expect: 3
    Result::Ok(())
}
```

What the rule costs is an impl like `Total` above, which needs a trait of
its own of the items. The way around it is to make the item type a
parameter of the type, so the bound is on a part of it:

```rust
trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

trait Total {
    effect e;

    fn total(&mut self) -> i64 with {Self::e, panic};
}

/// `item` holds a function giving a `T`, not a `T`: it's there to make
/// `T` a part of the type.
#[derive(Discard where I: Discard)]
struct W<I, T> { iter: I, item: fn() -> T }

impl<I, T> Total for W<I, T> where I: Iterator<Item = T>, T: Show + Discard {
    effect e = I::e;

    fn total(&mut self) -> i64 with {I::e, panic} {
        let mut n = 0;
        loop {
            match self.iter.next() {
                Option::Some(x) => n = n + x.show(),
                Option::None => return n,
            }
        }
    }
}

fn zero() -> i64 { 0 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(W { iter: 0..4i64, item: zero }).total())?; // expect: 6
    Result::Ok(())
}
```

A `where` clause on a function is never a problem: it's only ever assumed
in the function, and proved by its callers, never searched for.

## Packages that don't know each other

The other rules that were about the whole program already are about each
package, by the orphan rule (an impl is in the package of its trait or of
its type's head):

- A blanket impl, `impl<T: A> B for T`, is in `B`'s package, which `A`'s
  is, or depends on. So a cycle of blanket impls (`termination`), or of
  their associated types (`assoc_termination`), is inside one package.
- Two impls in packages that don't know each other can't overlap: one of
  a trait one of them declares can't be named by the other, and two of a
  trait both can name are each for a type of its own package.
- Proving something in one package never picks another's impl that it
  can't name, so a package checks the same alone as with any other.

That leaves the check that monomorphising ends.

## Which impl a call runs

Monomorphising makes a copy of each generic function for each type it's
used at. The checker rejects programs for which that would never end
(docs/04-generics.md): a graph with an edge for each use, weighed by how
much bigger the type gets, which must have no cycle that weighs more than
0. A call of a trait's method on a type parameter, `x.show()` with `x:
T`, was taken to go to every impl of the trait, as which one runs depends
on `T`. That's where the graph spanned packages.

But which impl runs was decided somewhere: where `T` was a type with a
head, the caller that proved `Option<i64>: Show` by `impl<U: Show> Show
for Option<U>`. That place can name the impl's type or trait, so it's in
the impl's package or one that depends on it. So the edge goes from there,
to the impl's *dictionary*, a node from which its methods are reached, as
a dictionary of methods is passed around in a dictionary-passing
semantics. This is more precise too: this was rejected before, as `helper`
calls `two` on a `U`, and `B`'s `two` calls back to `one`, though no `A`
is ever a `B`.

```rust
trait Tr1 {
    fn one(&self) -> i64;
}

trait Tr2 {
    fn two(&self) -> i64;
}

fn call1<W: Tr1>(w: &W) -> i64 { w.one() }

fn helper<U: Tr2>(u: &U) -> i64 { u.two() }

#[derive(Copy, Discard)]
struct A<T> { t: T }

impl<T: Tr1 + Copy> Tr1 for A<T> {
    fn one(&self) -> i64 { helper(&A { t: A { t: A { t: self.t } } }) }
}

impl<T> Tr2 for A<T> {
    fn two(&self) -> i64 { 1 }
}

#[derive(Discard)]
struct B<V> { v: V }

impl<V: Tr1> Tr2 for B<V> {
    fn two(&self) -> i64 { call1(&self.v) }
}

impl Tr1 for i64 {
    fn one(&self) -> i64 { *self }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(A { t: 5i64 }).one())?; // expect: 1
    out.line(&(B { v: 7i64 }).two())?; // expect: 7
    Result::Ok(())
}
```

What isn't decided where the impl is picked are a method's own type
arguments, given where it's called. With `Tr1 { fn one<K>(self, k: K) }`
and `Tr2 { fn two<N: Tr1, K>(self, n: N, k: K) }` in one package, a second
can write `impl<P: Tr2> Tr1 for Y2<P>`, whose `one` calls `self.x.two(self,
Wrap { k })`, and a third `impl Tr2 for X`, whose `two` calls `n.one(k)`.
Neither has a cycle, and together `Y2 { x: X {} }.one(0)` calls `two` with
a `Wrap<u8>`, which calls `one` with it, which calls `two` with a
`Wrap<Wrap<u8>>`, and so on, for ever. So a call on a type with no head
passes its own type arguments to one node for every impl there is, which
passes them on to every method's own type parameters, at the same size:
no package may pass a bigger one on to an impl it doesn't know.

```rust,error
trait Tr1 {
    fn one<K: Discard>(self, k: K);
}

trait Tr2 {
    fn two<N: Tr1, K: Discard>(self, n: N, k: K);
}

#[derive(Copy, Discard)]
struct Y2<P> { x: P }

#[derive(Discard)]
struct Wrap<K> { k: K }

impl<P: Tr2 + Copy> Tr1 for Y2<P> {
    fn one<K: Discard>(self, k: K) { self.x.two(self, Wrap { k }) }
}

fn main() {}
// error: polymorphic recursion: this call of `two` on a type parameter passes it `Wrap<K>` for `one`'s `K`
```

That's rejected alone, with no `X` anywhere: the rule is the package's.

### What a type needs

Copying a type copies what it mentions: its fields' types, and what its
projections are, `<W<i64> as Tr>::Out`, by the impl that's picked where
`S<W<i64>>` is named. Types and impls' associated types are nodes of the
graph too, with their own dictionaries, which reach only associated types.
And each copy of a type with a `discard` of its own makes a copy of that,
which runs wherever a value of it is dropped, so that's an edge too. It
wasn't before, and this was accepted, and monomorphising it never ended:

```rust,error
struct S<T> { t: T }

impl<T: Discard> Discard for S<T> {
    fn discard(self) {
        let _x = S { t: Option::Some(self.t) };
    }
}

fn main() {
    let _s = S { t: 1i64 };
}
// error: polymorphic recursion: this use of `S` with `T` = `Option<T>` leads back to `<S<T> as Discard>::discard`
```

## Sealed traits

A `sealed` trait can be implemented only by its own package:

```text
// in package `q`
export sealed trait Small {}
impl Small for u8 {}

// in a program that depends on `q`
impl q::Small for Mine {}
error: `Small` is sealed: only its own package, `q`, can implement it
```

So every package that can name it sees all its impls there will ever be,
and what has none never will. The overlap check uses that: two impls
whose types could be the same don't overlap if, where they would, one of
them needs a sealed trait of a type that no impl of it is for.

```rust
sealed trait Small {}

impl Small for u8 {}

impl Small for bool {}

trait Describe {
    fn describe(&self) -> i64 with panic;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 with panic { 1 }
}

impl Describe for char {
    fn describe(&self) -> i64 with panic { 2 }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> i64 with panic {
        match self {
            Option::Some(x) => 10 + x.describe(),
            Option::None => 10,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&'a'.describe())?;                // expect: 2
    out.line(&Option::Some(true).describe())?; // expect: 11
    Result::Ok(())
}
```

Without `sealed`, another package could implement `Small` for `char`, and
the two impls overlap. It's one step only: a type no impl's type could
be. A sealed trait with a blanket impl, `impl<T: Open> Small for T`, rules
nothing out.

`Int`, `Signed` and `Unsigned` are sealed. So core has one `impl<T: Int>
Ord for T` and one `impl<T: Int> Display for T`, where it had ten of each,
and a library can have one impl for every integer type beside its others.
To hash any integer, or print its bits, there's `core::num::wrap`: `x`
modulo 2^w, for a `U` that's `w` bits wide, in `U`'s range.

```rust
use core::num::wrap;

trait Hash {
    fn hash(&self) -> u64;
}

impl<T: Int> Hash for T {
    fn hash(&self) -> u64 { wrap(*self) }
}

impl Hash for bool {
    fn hash(&self) -> u64 {
        if *self { 1 } else { 0 }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(-1i8).hash())?;        // expect: 18446744073709551615
    out.line(&wrap::<u16, u8>(300))?; // expect: 44
    out.line(&wrap::<u8, i8>(200))?;  // expect: -56
    out.line(&true.hash())?;          // expect: 1
    Result::Ok(())
}
```

Adding an impl of a sealed trait is a breaking change: code in other
packages may count on a type not having one.

## Why it's sound

- **C1, closed traits are closed.** A proof of `t: C`, for `C` closed,
  needs only proofs of closed traits, of parts of `t`. `Discard`'s written
  impls may need only `T: Discard` of their parameters
  (`written_discard`), and a derived `Discard` or `Copy` only the same of
  its parameters (`derive`); a closure is `Discard` or `Copy` if what it
  captures is; the `Fn` traits are the checker's, of functions, closures
  and references to them; the integer traits are the checker's, of the
  integer types, and need nothing.
- **L1, proving ends.** A step of a proof by an impl for more than a bare
  parameter needs bounds on its parts, smaller types; one by a blanket
  impl is about the same type, lower down the blanket impls, which have
  no cycles (`termination`); one on what isn't a part is of a closed
  trait, a search of its own that ends by C1 and comes back to no other
  trait. Ordered by the type's size and then the blanket impls, every
  path of a proof is finite. Each impl keeps the rule alone.
- **O, the orphan rule makes the rest local.** A blanket impl is in its
  trait's package, so an edge between traits by one goes to a package the
  same or depended on, and a cycle of them is in one package. Impls in two
  packages that don't know each other can't overlap, nor can a package's
  proofs pick an impl whose trait and head it can't name.
- **R1, the impl a call runs is picked by a proof the graph has.** At a
  call on a type with a head, the impl is picked there. On a type
  parameter, its value came from a caller, and the impl from the first
  that proved the bound at a type with a head, or from an impl's own
  proof of a supertrait or of what an associated type must implement, or
  from an opaque type's function. Each of those has an edge to the impl's
  dictionary, with the impl's type parameters as that place has them, and
  monomorphisation copies the method with those. So every copy is at the
  end of a path of the graph, and its type arguments are no bigger than
  the path's weight says, which is the argument of docs/04-generics.md.
  Each edge goes from a package to one it depends on.
- **R2, unknown impls can't make a cycle grow.** A method's own type
  arguments at a call on a type with no head go to `UNKNOWN`, and from it
  to every method's own type parameters, weighing 0. Each package rejects
  any cycle through `UNKNOWN` that grows, so every path in it from a
  method's own type parameter to such a call weighs at most 0. A cycle of
  the whole program is made of paths that don't go through `UNKNOWN`,
  each in one package and its dependencies, as edges only go there, and
  of steps through `UNKNOWN`: so it weighs at most 0. A dependency added
  to a program adds no edge from what was there, but to its own methods
  through `UNKNOWN`, which it checked.
- **R3, every copy monomorphisation makes is in the graph.** Functions
  from calls and paths; types from what each copy mentions, in its
  signature, its type arguments and its hidden types, and from what each
  type's fields and projections mention; and a `discard` of its own with
  each copy of its type (`discard_uses`).
- **S1, a sealed trait's impls are complete.** Only its package
  implements it, which every package that can name it is or depends on,
  and overlap is checked after all of a package's impls are collected.
- **S2, ruling an impl out is sound.** If no impl of a sealed trait, nor
  the checker, is for a type that unifies with `t`, then none is for any
  type `t` can become: so `t: S` can't be proved of any of them. If two
  impls' types unify, and one is ruled out where they do, it's ruled out
  of every type both match, so at most one impl is found for any type.
  Generic code finds the impl a bound's proof found, and monomorphisation
  the same one, as being ruled out is kept by substitution.
- **W, `wrap` is one intrinsic.** It gives what its type says, a `U`, and
  the interpreter checks it's in `U`'s range.

## Design decisions worth noticing

- **A closed set, not a closed-world analysis.** Which traits are closed
  is a fixed list of the checker's, not worked out from the impls there
  are: then a new impl somewhere could make a trait not closed, which is
  the whole-program reasoning this step removes. A use-site check, one
  that looks for a loop where a bound is proved, would accept more; the
  rule here accepts less than it would, so moving to one later breaks
  nothing.
- **Dictionaries, not dispatch.** The graph follows which proof picked an
  impl, as dictionary-passing would, rather than every impl a call could
  run. It's local, and more precise.
- **One node for every unknown impl.** It's the most a package can say of
  impls it doesn't know: that they pass a method's own type arguments on
  as they are. A method that wraps its own type argument for another
  trait's method on a type parameter is rejected, though no impl in the
  program might make that loop.
- **`sealed` is a keyword.** It changes what the checker may reason about,
  and who may write impls, as `export` changes who may name an item.
- **One step of ruling out.** Only whether an impl's type could be the
  type, not what that impl in turn needs: that would be a search of its
  own, and one step covers the integer types.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| a bound of a user's trait on what isn't part of an impl's type | needs use-site resolution, with a check that ends; this rule is a subset of that | — |
| ruling an impl out through what another impl needs | one step covers what's needed; deeper needs its own termination argument | — |
| sealed traits a package's own code can't implement either (Rust's `#[non_exhaustive]`-like) | no use yet | — |
| `wrap` for `char` and `bool` | an integer type's bits are what's asked for; `as` converts those | — |

## Exercises

1. In `closed_bounds`, let a bound of a sealed trait whose every impl
   needs only closed traits through. Is that sound? What would a new impl
   in its package do to the packages that depend on it?
2. Write the three packages of the `Y2` example, with `Y2`'s `one`
   passing `k` as it is. Which is the cycle now, and why does it weigh 0?
3. Take the edges from `discard` out (`discard_uses`). Which test hangs,
   and where: in the checker, or after it?
4. Write `impl<T: Signed> Sign for T` and `impl Sign for u8`. Why don't
   they overlap, when `impl<T: Int> Sign for T` and `impl Sign for u8`
   would?
5. Why must a sealed trait's package check its impls for overlap after
   collecting all of them? What could go wrong if it were done one module
   at a time?

## Reading

- Ross Paterson and others, the *Paterson conditions* in GHC's user guide:
  what makes instance resolution end, which closed traits extend.
- Philip Wadler and Stephen Blott, *How to make ad-hoc polymorphism less
  ad hoc* (1989): dictionary-passing, which the graph's edges follow.
- Rust RFC 1023, *Rebalancing Coherence* (2015), and RFC 1210,
  *Specialization*: negative reasoning, and why it needs a closed world,
  which a sealed trait is.
- Scala 3's and Kotlin's `sealed`: the same word, for subclasses rather
  than impls.
