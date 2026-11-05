# Step 45: variants by their names

Code: `Parser::pat_atom`, `Parser::variable` and `starts_upper` in
`src/parser.rs`; `PatternKind::Path` in `src/ast.rs`; `Item::Variant`,
`Resolver::enum_at`, `Resolver::variant_leaf`, `Rewrite::variant_named`,
`Rewrite::pattern_path` and `Rewrite::no_variant` in
`src/check/modules.rs`; `core/prelude.t`.

Step 2 made a variant always qualified, `Shape::Empty` and never
`Empty`, for a good reason (see docs/02-adts.md): in Rust,
`match x { Empty => .. }` with no `Empty` in scope binds a new variable
called `Empty`, which matches everything, and the arms after it never
run. Retrospective 4 counted what the rule costs: `Option::` and
`Result::` 141 times in t-nonograms' 1,693 lines, 56 times in tcalc's
536. Most of a `match` on an `Option` was the word `Option`.

This step keeps the reason and drops the cost. Case decides, in a
pattern only: a name that starts with an uppercase letter is a path,
never a new variable. Then variants can be brought in by `use`, and the
prelude brings in the four everyone uses.

```rust
use Shape::{Dot, Empty};

#[derive(Discard)]
enum Shape { Empty, Dot(i64) }

fn size(s: Shape) -> i64 {
    match s {
        Empty => 0,
        Dot(_) => 1,
    }
}

fn half(n: i64) -> Option<i64> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let h = match half(8) {
        Some(h) => h,
        None => 0,
    };
    out.line(&(size(Dot(3)) + size(Empty) + h))?;
    Ok(())
}
// expect: 5
```

## The case rule

In a pattern, a name that starts with `A` to `Z` names a variant or a
const: `None`, `Dot(p)`, `MAX`. It's resolved like any other name, in
the module's namespace for values, and must be found:

```rust,error
#[derive(Discard)]
enum Shape { Empty, Dot(i64) }

fn size(s: Shape) -> i64 {
    match s {
        Empty => 0,
        // error: `Empty` isn't a variant in scope: write `Shape::Empty`, or bring it in with `use Shape::Empty;`
        Shape::Dot(_) => 1,
    }
}
```

The other half of the rule is what makes the first safe: no variable's
name can start with an uppercase letter. That covers every place a
variable is bound: `let`, `match` arms, `for`, a function's or a
closure's parameters, and the shorthand `Point { x }`.

```text
fn double(N: i64) -> i64 {
          ^ a variable's name can't start with an uppercase letter: a name
            like `N` is a variant's or a const's (fix: call it `n`)
```

And no variant's name can start with anything else, nor can the name a
`use .. as` gives a variant, as a lowercase variant in a pattern would
be the variable the rule is there to rule out:

```text
enum Light { red }
             ^^^ a variant's name starts with an uppercase letter: in a
                 pattern, `red` would be a new variable (fix: call it `Red`)
```

The naming lints said all this before, as warnings: types and variants
in `UpperCamelCase`, variables in `snake_case`. Code that followed them
doesn't change. What's new is that, for variants and variables, case
is grammar: what a lint suggested is now what a name *means*.

> **Lemma V2 (no capture).** No variable's name starts with an
> uppercase letter, so resolving such a name to a variant, in an
> expression or a pattern, never takes the place of a variable: the
> rewrite below changes only names that couldn't have meant one. A
> name that starts lowercase is never rewritten.

> **Lemma V3 (no silent binding).** A pattern's name that starts with
> an uppercase letter is a variant, or it's an error. Every variant's
> name, and every name a variant is brought in as, starts with an
> uppercase letter. So a variant can never be written where a variable
> would be bound instead: Rust's `Empty =>` mistake has no spelling in
> T.

Names starting with `_` follow the lowercase side: `_unused` is a
variable, as before. A variant whose name started with `_` to keep
`dead_code` quiet is now an error; `#[allow(dead_code)]` on the variant
does that instead.

## Bringing variants in

A variant is an item in the values namespace, as a function is, but in
no module's by itself: only a `use` puts one there. Its path goes
through its enum, which can be named in any way a type can:

```rust
mod lights {
    #[derive(Discard, Copy, Eq)]
    pub enum Light { Red, Amber, Green }

    pub use Light::{Amber, Green, Red};

    pub fn next(l: Light) -> Light {
        match l {
            Red => Green,
            Green => Amber,
            Amber => Red,
        }
    }
}

use lights::Light::Red as Stop;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let go = lights::next(lights::next(Stop)) == lights::Amber;
    out.line(&go)?;
    Ok(())
}
// expect: true
```

A variant is seen as far as its enum is: a `pub use` of a private
enum's variants is an error, as a `pub use` of the enum would be. A
module's own names come before the prelude's, as for every name, so a
module that declares or imports a `None` of its own means that one.

The prelude's four are plain `export use`s in `core/prelude.t`:

```text
export use crate::option::Option::{None, Some};
export use crate::result::Result::{Err, Ok};
```

No name is built in: an `Option` of a program's own, and its variants,
are named as any enum's are, and `Option::Some` still means
`core::option::Option::Some` wherever the prelude's `Option` is in
scope.

> **Lemma V4 (resolution is a function of the program).** A variant is
> a binding like any, brought in only by a `use` that names it, with
> no glob imports: lemma M1 of docs/06d-modules.md holds as it did. A
> name a module declares or imports itself comes before the prelude's,
> so adding to the prelude can only turn an error into a meaning, never
> one meaning into another.

## Resolving is rewriting

Resolving a module's paths rewrites each name that turns out to be a
variant into the qualified form steps 2 to 44 know:
`Dot(3)` becomes `crate::Shape::Dot(3)`, `None` in a pattern
`core::option::Option::None`, the enum's path marked as already
resolved (by its node, `Modules::paths`) so it's never looked up again
where the enum's own name may not be in scope. In an expression only a
name with no variable, type or const parameter, or nested function of
that name in scope is looked at, which by lemma V2 means only a name
that starts with an uppercase letter and isn't one of the last three.

> **Lemma V1 (nothing else changes).** After resolving, every variant
> is written as it was in 0.60.0: the checker, the borrow checker, MIR,
> the interpreter and the C backend see nothing new, and every lemma
> about them holds as it did.

## Consts in patterns, not yet

The case rule makes room for a const in a pattern: `MAX =>` is now
unambiguously a path. What it would mean isn't decided (a test against
an opaque value, which no `match` could count as exhaustive), so for
now it's an error that says so:

```rust,error
const LIMIT: u8 = 3;

fn at_limit(x: u8) -> bool {
    match x {
        LIMIT => true,
        // error: `LIMIT` is a const: a pattern can't name one yet
        _ => false,
    }
}
```

Adding them later won't change the meaning of any program that checks
now.

## Design decisions worth noticing

- **Case as grammar, in patterns only.** T's naming rules are lints:
  how a function or a type is spelled never changes what it is. The
  exception is narrow, the one place where Rust shows that case can't
  be left to convention. The alternatives were Swift's `.Some(x)`,
  resolved by the expected type (a new kind of name lookup, which needs
  inference before resolution), and special-casing `Option` and
  `Result` (a privilege no other enum could have).
- **Every variable, not only those of patterns.** A parameter called
  `N` would make `N` in the body a variable or a variant depending on
  what's in scope, which is what lemma V2 rules out. It also leaves the
  rewrite free of scopes: it needs to know which names are parameters
  of the items around it, and which functions a block declares, nothing
  else.
- **The type arguments are the enum's.** `None::<i64>` is an error
  that says `Option::<i64>::None`: a variant has no type parameters of
  its own, and a bare one has nowhere to put them.
- **Everything ported.** The libraries, examples and tests say `Some`,
  `None`, `Ok` and `Err`, and the programs that use T import the
  variants they match most. Chapters before this one keep their
  qualified examples, which still mean what they did.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| consts in patterns | a test against an opaque value needs a design | — |
| `use Shape::*` | glob imports would make resolution depend on order (lemma M1) | — |
| a variant as a function value, `map(Some)` | a closure `\|x\| Some(x)` does it; closures are their own types | — |
| struct-like variants, `Move { x, y }` | variants hold tuples | — |
| a lint for a qualified variant that's in scope bare | style; the qualified form stays valid | — |

## Exercises

1. Why would lemma V2 fail if only patterns followed the case rule?
   Write a function whose meaning would depend on an import.
2. A module declares `fn Some(x: i64) -> i64`. What does `Some(1)` mean
   in it, and what does `Some(x) =>` mean in a pattern there?
3. Why may `use Shape::Empty as Nothing;` bring in a variant but
   `use Shape::Empty as nothing;` not?
4. `let Total = 5;` is an error. Which half of the rule rejects it, and
   what does the fix change?
5. What would a const pattern have to mean for a `match` on a `u8`
   with arms `LIMIT`, `0..=2` and `4..` to be exhaustive? Why can't the
   checker know?

## Reading

- The Rust Reference, *Identifier patterns*: when a name in a pattern
  binds and when it refers to a constant or unit variant, which depends
  on what's in scope.
- Haskell 2010 Report, section 2.4: constructors and variables are told
  apart by the case of their first letter, as T now does in patterns.
- The Swift Programming Language, *Enumerations*: `.north`, a case
  named by its type, inferred from context.
