# Step 17: what the first retrospective changed

Code: `Generics::effects` in `src/ast.rs`, `types::Params`,
`generics::declare` and `resolve_effects` in `src/check/generics.rs`,
`Checker::projection` in `src/check/traits/bounds.rs`,
`collect_impls_in` and `derive_fix` in `src/check/traits/collect.rs`,
`Checker::derive`, and `Parser::inner_doc` in `src/parser.rs`.

After step 16, a review looked back over the rules so far, for the ones
that say more than they need to, or hold only because of how a program
happens to be written. Three were small enough to change at once. Each is
a breaking change, and each makes a rule local: what a name means is
known from its declaration, what makes a type `Discard` from its fields,
and what documents a module from the module.

## Effect parameters are declared with `effect`

Until now an effect parameter was told apart from a type parameter by
case: `e` in `fn apply<e, F: Fn() with e>` was an effect parameter
because it was lowercase. But case is a lint everywhere else
(`non_snake_case`, `non_camel_case_types`), never what a name means, so
`fn id<t>(x: t) -> t` was an error, "`t` is an effect", and core's `fn
map<U, f, F: FnMut(Self::Item) -> U with f>(self, f: F)` had an effect
`f` and a value `f` that only case told apart from types. Now the keyword
says it:

```rust
fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

// A type parameter in lowercase is a type parameter, with a warning.
#[allow(non_camel_case_types)]
fn id<t>(x: t) -> t { x }

fn main() {
    print(apply(|| 3)); // expect: 3
    print(id(4i64));    // expect: 4
}
```

> **Lemma K1 (kinds).** Every type, effect, associated type and
> associated effect parameter has the kind its declaration gives it:
> `effect e` and `effect e;` an effect, anything else a type. A name in
> a type resolves only to one of kind type, and a name in a `with` only
> to one of kind effect; anything else is an error. So a name's kind never
> depends on how it's spelled, and substitution maps each parameter to an
> argument of its kind.

The checker keeps the kind with each list of parameters (`types::Params`),
from the declaration through instantiation, impl selection and
monomorphisation, where each used to ask `is_effect_name`. Where a kind
can be got wrong, it's checked:

```rust,error
// `E` is declared a type parameter, so it can't be in a `with`.
fn apply<E, F: Fn() with E>(f: F) with E { f() }

fn main() { apply(|| {}) }
// error: `E` is a type parameter, not an effect; declare it as `effect E`
```

An impl's method must take parameters of the kinds the trait's method
does, as the trait's callers give it arguments of those kinds:

```rust,error
trait Tr {
    fn m<A: Discard>(&self, a: A) -> i64;
}

#[derive(Discard)]
struct W {}

impl Tr for W {
    fn m<effect a>(&self, x: i64) -> i64 with a { x }
}

fn main() {}
// error: its `a` is an effect parameter, where the trait's `A` is a type parameter
```

The same went for associated items, where it was a hole: an associated
type could be used as effects, and an impl could give
an associated effect with `type e = i64;`. Version 0.28.0 accepted both,
and then said things like "`go` may have the effects `i64`". Now:

```rust,error
trait Tr {
    type Item;
    fn run(&self) with Self::Item;
}

fn main() {}
// error: `Item` is an associated type of `Tr`, not an effect
```

`panic` and `alloc` can't be effect parameters' names, as `with panic`
would then mean two things; an effect parameter goes anywhere among the
type parameters, after the lifetimes, and still has no bounds.

## `Discard` and `Copy` are only derived

A written `impl Discard for W<T>` could need anything of `T`, so proving
a type `Discard` could need another trait, and through that trait's
impls, lead anywhere. Step 14 found one place this mattered: core's `impl
Iterator for Filter<I, P>` needs `I::Item: Discard`, and an impl
`impl<T: Iterator + Discard> Discard for W<T>` in a program made proving
`Iterator` able to go on forever, through two packages. Now `Discard` and
`Copy` can only be derived:

```rust,error
struct W<T> { t: T }

impl<T: Iterator + Discard> Discard for W<T> {}

fn main() {}
// error: `Discard` can only be derived, with `#[derive(Discard)]` on a struct or enum, so that whether a type is `Discard` follows from its fields alone
```

The fix derives it instead. Nothing is lost by it: the traits have no
methods, so a written impl could only differ from the derived one in what
it needs, and the derive works out the least that makes every field
implement it (docs/06-traits.md).

> **Lemma D1 (marker proofs are closed).** A proof of `t: Discard` or
> `t: Copy` needs only obligations `u: Discard` or `u: Copy`, of the same
> trait, never another. By induction on the proof: the built-in impls
> (integers, `bool`, `char`, `()`, `!`, references, functions, slices'
> references, `Box<T>`) need at most the same trait of their arguments,
> and every other impl is derived, which needs the same trait of each
> field's type; such a field obligation, by the induction, needs only the
> same trait, so the derive's bounds, the least fixed point, are bounds of
> that trait on the type's parameters.

So proving `Discard` never reaches `Iterator`, or any trait a program
declares, and the cycle through core above can't be made. What a derived
impl says it needs, in a `where`, can only be the same trait too, as
anything else would be more than its fields could need:

```rust,error
pub trait Show {
    fn show(&self) -> i64;
}

#[derive(Discard where T: Show)]
pub struct W<T> { t: T }

fn main() {}
// error: a derived `Discard` can only need `Discard` of type parameters: that's all its fields can need
```

`Eq` stays as it was: it has a method, so a written impl says something a
derive can't, and a derived `Eq` can still need another trait of a type
parameter, if a field's `Eq` does.

## `//!` documents a module

A file's first lines described it, as plain comments, so nothing could
show them. Now `//!` lines at the start of a module, before anything else
in it, are its doc comment, as in Rust: at the top of a file for the
module that is the file, or just after the `{` of `mod m {`. Hovering a
module's name shows them, where it's declared, in a `use`, and at the
start of a path, and the libraries' files start with theirs.

```rust
//! A program with a module.

mod shapes {
    //! Shapes, and what they're made of.

    /// A point.
    #[derive(Discard)]
    pub struct Point { pub x: i64 }
}

fn main() {
    let p = shapes::Point { x: 1 };
    print(p.x); // expect: 1
}
```

There's one way to say it. A `///` before `mod m` would document it from
outside, which a module in a file of its own can't, so it's an error,
whose fix, for an inline module, moves it in as `//!`:

```text
/// Shapes.
mod shapes {}
// error: a module's doc comment goes inside it, as `//!` lines at the start
```

A `//!` anywhere else documents nothing, and gets the `unused_doc_comments`
warning a stray `///` gets. `t fmt` prints a module's `//!` lines first in
it, then a blank line.

## Design decisions worth noticing

- **Kinds by declaration.** Rust tells lifetimes apart by their `'`, and
  everything else by what's declared; T now does the same. Case is only a
  convention, kept by lints with fixes.
- **A restriction, not a check.** The termination check of step 14 stays,
  but it no longer has a case to catch through `Discard`. Making the
  markers derive-only closes the cycle by construction, which no order of
  checking packages could get wrong.
- **One place for a module's docs.** Both `///` before and `//!` inside
  would be two ways to say one thing, and only one works for a module in
  a file.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `Eq`, `Copy`, `Discard`, `Fn*` as items of core | built in, as global names; they move into core, with `Box` into alloc | 17.1 |
| writing an effect argument, `apply::<effect {panic}, _>` | `_` is enough while arguments are inferred | — |
| hovering a path's middle segments, `a::b::C`'s `b` | the parser doesn't keep where each segment is | — |

## Exercises

1. Lemma D1 lists the built-in impls. Which of them would break it if
   `Box<T>` were `Discard` only when `T: Copy`? Find a program that the
   termination check would then have to reject.
2. Why can't an effect parameter have bounds, when an associated effect
   can be set by an impl? What would `effect e: panic` mean, and is it a
   bound or an equality?
3. Write a module in a file of its own with a `//!` doc comment, and a
   program that hovers it through a `use`. What does going to its
   definition do?

## Reading

- Benjamin Pierce, *Types and Programming Languages* (2002), chapter 29,
  *Type Operators and Kinding*: kinds as the types of types, given by
  declarations and checked like types are.
- The Rust reference, *Doc comments* and *Generic parameters*: `//!`,
  and how Rust tells lifetime, type and const parameters apart.
- Ralf Jung and others, *RustBelt* (2018), on marker traits whose
  meaning is structural, as `Send` and `Sync` are, and why they're unsafe
  to implement by hand.
