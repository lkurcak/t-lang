# Step 25: type aliases

Code: `AliasDecl` in `src/ast.rs`, `AliasDef` and its `expand` in
`src/check/generics.rs` (what a use means), and `alias_cycles`,
`check_aliases` and `alias_body` in `src/check/adts.rs` (the rules a
declaration keeps).

A type can be long to write, and the same long type is often written many
times: tcalc's parser takes a `&'a Arena<Expr<'a>>` in three places. A
*type alias* gives it a name:

```rust
use alloc::arena::Arena;

#[derive(Discard)]
enum Expr<'a> { Num(i64), Neg(&'a Expr<'a>) }

/// Where a parser's expressions live.
type Nodes<'a> = Arena<Expr<'a>>;
type Pair<T> = [T; 2];

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Neg(inner) => 0 - eval(inner),
    }
}

fn negated<'a>(nodes: &'a Nodes<'a>, n: i64) -> &'a Expr<'a> with alloc {
    nodes.alloc(Expr::Neg(nodes.alloc(Expr::Num(n))))
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let nodes: Nodes = Arena::new();
    let p: Pair<i64> = [eval(negated(&nodes, 3)), 4];
    out.line(&(p[0] + p[1]))?; // expect: 1
    Result::Ok(())
}
```

An alias is another name, not another type: `Nodes<'a>` *is*
`Arena<Expr<'a>>`, and either can be used where the other is. A new type
that can't be mixed up with what it holds is a struct with one field.

## What a use means

A use of an alias means what it stands for, with each of its parameters
replaced by the argument written there. Its lifetime arguments can be
left out, as a struct's can, and are then elided as theirs are:
`nodes: Nodes` in `main` above.

The names in what it stands for mean what they mean where the alias is
declared, not where it's used. `m::P` below is `m`'s `Point`, in a module
with a `Point` of its own:

```rust,error
mod m {
    pub struct Point { pub x: i64 }
    pub type P = Point;
}

struct Point { x: bool }

fn main() {
    let _p: m::P = Point { x: true };
}
// error: these are two different types called `Point`
```

The checker resolves the names of every type written in the program by
the node they're written at, before checking starts (`check/names.rs`), so
that comes for free: a use resolves the alias's own nodes, which were
resolved where it's declared. The parameters are what differs from use to
use. While what an alias stands for is resolved for a use, each of its
parameters' names means the argument, already resolved
(`TypeName::Arg`), and each of its lifetimes the region given for it
(`Lifetimes::Alias`).

So a projection in it, `<I as Iterator>::Item`, is worked out at the use,
for the type given for `I`, under the bounds there. And a bound that what
it stands for needs is checked there too:

```rust,error
struct Sorted<T: Ord> { t: T }
type S<T> = Sorted<T>;
struct N {}

fn f(_s: S<N>) {}
// error: `Sorted` needs its `T` to implement `Ord`, and `N` doesn't
```

## The rules

An alias is declared at the top of a module, or in a block, as a struct
can be. Each rule below is what the argument for soundness, after them,
needs.

- **R1. No alias contains itself**, even through others:
  `type Chain = Option<Box<Chain>>` is an error. A recursive type needs a
  struct or enum, which is a type of its own, so `Box<Node>` stops there.
- **R2. Its names mean what they do where it's declared.**
- **R3. It's closed**: what it stands for names no lifetime but its own
  and `'static`, and leaves none out, and it uses every parameter. One it
  didn't use couldn't be worked out from anything.
- **R4. Its parameters have no bounds.** A bound would be a promise that
  nothing checks, as Rust's are: what an alias stands for is checked at
  each use instead, as if written there.
- **R5. It's seen no further than what it stands for.** A `pub` alias
  names only what's seen in the whole package, and an `export` one only
  what's exported, as a `pub use` of a private item is an error:

```rust,error
mod m {
    #[derive(Discard)]
    struct Secret { n: i64 }
    pub type Open = Secret;
}
// error: `Open` is public, but what it stands for names `Secret`, which is private to `m`
```

- **R6. Nothing in it depends on where it's used**, besides its
  parameters: no `impl Trait`, no `_`, and a projection names its trait.
  Which trait `T::Item` is of depends on the bounds where it's written, and
  an alias has none.

And it stands for a type: not in a struct literal, a pattern or a path
like `Shape::Dot`, where the struct or enum is written instead. The error
says what to write. It can't stand for `str` or `[T]`, which have no size
of their own; an alias of `&str` can.

## Why it's sound

**A1. Transparency.** Resolving a use of an alias gives the type that
resolving what it stands for would give, written at the use, with each
parameter replaced by its argument, and each name resolved where the
alias is. So every check after resolving (unification, bounds, the orphan
rule, effects, refinements, the borrow checker, monomorphisation) sees
exactly the program with its aliases written out by hand, and each
argument that program is sound already applies. An alias adds nothing to
trust.

That's why the rules are as they are. R2 makes "written out" mean one
thing for every use. R3 and R6 make an alias's meaning a function of its
arguments alone: written out at two uses with the same arguments, it's
the same type. R4 keeps the alias from saying anything that the checks of
the written-out program don't check.

**A2. Expanding ends.** By R1, aliases name each other acyclically, so
the depth of aliases in aliases is at most how many there are.

**A3. What can't be named stays so.** By R5, whoever can name an alias
can name everything in what it stands for, so an alias never lets a
module or package name what it couldn't before. Abstraction doesn't
change: whether a field can be read is the struct's business, and an
alias of the struct is the struct (`field \`n\` of \`S\` is private to \`m\``
still).

**A4. Packages are still checked apart** (docs/14-packages.md). An
exported alias's expansion is part of its package's interface, and by R5
that's made only of exported items.

## Design decisions worth noticing

- **Expanded where types are resolved**, not rewritten in the source. The
  monomorphiser resolves the types it copies through the same function,
  so it sees no alias either, and nothing after it needs to know they
  exist.
- **Messages show what an alias stands for**, as the checker knows types
  only after resolving them. An editor's hover shows the alias, and what
  it stands for at that use.
- **Bounds on parameters wait for a rule that checks them**: at the
  declaration, that what it stands for is well formed under them, and at
  each use, that the arguments meet them.
- **An unused alias is dead code**, and one is used if anything names it,
  even something dead: an alias is gone once resolved, so it can't be
  followed from `main` the way functions are.

## Deliberately missing (coming later)

- Refinement aliases, `type Pos = { v: i64 | v > 0 };`: a refinement is
  only the whole type of a parameter, result, `let` or field, and its
  predicate's nodes would need copying for each use.
- Effect parameters on aliases, `type Callback<effect e> = fn() with e;`.
- Aliases of `str` and `[T]` behind a reference, `&Bytes`.
- Aliases in struct literals, patterns and paths.
- Checked bounds on an alias's parameters.
- Trait aliases.

## Exercises

1. Turn off R1 and find what loops. Where would a cycle through a struct
   be caught instead?
2. Why would an unused parameter be unsound in no way at all, yet still
   an error? What would `Count<_>` mean?
3. Allow bare projections in aliases, and write two uses of one alias
   that mean different things.
4. An alias of a function type, `type Op = fn(i64) -> i64;`, has no
   `with`. What effects does it allow? Why is that the same as the type
   written out?
5. Make error messages show aliases as written. What would `Type` have to
   carry, and what would unification do with it?

## Reading

- The Rust reference's *Type aliases*, and the `type_alias_bounds` lint
  for why bounds on them mislead.
- Haskell's `type` declarations, and the `LiberalTypeSynonyms` and
  `TypeSynonymInstances` extensions, for how far synonyms can go.
- OCaml's manifest types (`type t = int`) against abstract ones
  (`type t`), in Leroy, *Manifest types, modules, and separate
  compilation* (1994).
