# Step 67: refinement aliases and variants' fields

Code: `src/check/written.rs` (`Checker::written_out`),
`AliasDef::refined`/`refinable` and the `At` of `resolve_in` in
`src/check/generics.rs`, `Checker::alias_shapes` in `src/check/adts.rs`,
and in `src/check/refine.rs` `Checker::alias_now`,
`Checker::variants_now`, `RefinedVariants`, `Walk::need_variant` and
`Walk::assume_variants`; `Walk::variant_constrained` in
`src/check/refine/mir.rs`, `Walk::own_tree` in
`src/check/refine/targs.rs`, and `Interp::check_variant`.

A refinement had to be written out wherever it was needed: t-chess wrote
`{ s: u8 | s < 64 }` 28 times. And an enum's variant couldn't refine its
fields at all, so t-libs' UTF-8 decoder masked a number before shifting
it, to tell the checker what it couldn't be told. Both can be said now:

```rust
type Square = { s: u8 | s < 64 };

#[derive(Copy, Discard)]
enum Move {
    Step(Square, Square),
    // The second field's refinement names the first, by its binder.
    Slide({ from: u8 | from < 56 }, { to: u8 | to > from && to < 64 }),
}

fn need(_s: { s: u8 | s < 64 }) {}

fn target(m: Move) -> Square {
    match m {
        Move::Step(_, to) => to,
        Move::Slide(_, to) => to,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let m = Move::Slide(8, 16);
    need(target(m));
    out.line(&target(Move::Step(1, 2)))?; // expect: 2
    out.line(&target(m))?;                // expect: 16
    Ok(())
}
```

## Refinement aliases

A type alias whose type has a refinement in it, where one can be, is a
*refinement alias*. A use of one means what the refinement written out
there would: the parameter's, result's, `let`'s or field's refinement,
or a type argument's, as deep as it's used. So `List<Square>` is a list
of squares, `Option<Square>` an option of one, and `{ s: Square | s != 0 }`
both refinements at once.

Aliases can take parameters, and a const parameter can be in the
predicate:

```rust
use alloc::list::List;

type Below<const N: usize> = { i: usize | i < N };
type NonEmpty<T> = { l: List<T> | l.len() > 0 };

fn get<const N: usize>(a: &[u8; N], i: Below<N>) -> u8 { a[i] }

fn last<T: Copy>(l: &NonEmpty<T>) -> T { l[l.len() - 1] }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l = List::new();
    l.push(get(&[4u8, 5, 6], 2));
    let l: NonEmpty<u8> = l;
    out.line(&last(&l))?; // expect: 6
    Ok(())
}
```

The rules are the type alias's (docs/25-type-aliases.md, R1 to R6), and
three more, each what the argument below needs:

- **R7. Its predicates are closed.** They name their binders, the alias's
  const parameters, and consts, as they are where the alias is declared:
  nothing where it's used. An alias's meaning is a function of its
  arguments alone (R3, R6), and a name of the use's would make it depend
  on where it's used.
- **R8. It's used only where a refinement can be.** Not in a `fn` type, a
  call's type arguments, an impl's header or an associated type: the
  error says the alias stands for a refinement. A parameter of an alias
  that's somewhere a refinement can't be, in what it stands for, can't
  take a refined argument either: `type F<T> = fn(T) -> u8;` can't be
  `F<{ v: u8 | v < 3 }>`.
- **R9. It's checked where it's declared.** Its predicates are checked
  once, for any arguments, in its module: its parameters have no bounds
  (R4), so `type Pos<T> = { v: T | v > 0 }` is an error, as `v > 0` is no
  predicate for a `T` that might not be an integer. And what they name is
  seen as far as the alias is (R5): a public alias's predicate can't name
  a private field.

```rust,error
type Below = { v: u8 | v < n };

fn f(n: u8) -> u8 {
    let x: Below = 0;
    x + n
}

fn main() {
    let _ = f(3);
}
// error: cannot find value `n` in this refinement
```

## Written out

What a type is doesn't need anything new: resolving a use of an alias
resolves what it stands for, and where a refinement can be, resolving
leaves it out (`At::refined`), as it always did for a refinement written
there. But refinements aren't in `Type`. The refinement checker reads
them off the type as written (`TypeExpr::refinements`), each at a
position of it, and keeps what it learns of each predicate (which value
each name in it is, which ways into a value it's of) by the predicate's
node. So it reads them off a copy, the type *written out*
(`Checker::written_out`): each refinement alias in it, and each alias given
a refined argument, replaced by what it stands for, with each type
parameter replaced by its argument (written out) wherever it is, each
const parameter in a predicate by what its argument is, and each
predicate a copy of new nodes. Monomorphisation and the MIR's checks for
the interpreter read the same copy (`Info::written`).

The type's names in the copy keep their nodes, so they mean what they do
where they're written, as R2 says. A predicate's names are looked up by
name, not node: a predicate copied from an alias is checked in the
alias's module, naming only its binder and consts there, and an argument
given for a const parameter is put in as what it named where it was
written (`Fixed`), so no name in the alias catches it, nor any of the
use's an alias's name.

> **Lemma A5 (written out).** The refinements the checker reads off
> `written_out(t)` are those of `t` with each alias written out by hand:
> the same predicates, meaning the same, at the same positions. By
> induction on how deep aliases are in aliases, which ends (A2). A
> parameter is only where a refinement can be (R8), so its argument's
> refinements are at each of its positions, as written out by hand they
> would be; where it isn't, nothing is read, and resolving already
> refused a refined argument there. A copied predicate's names are what
> they are where it was written: its binder, which shadows, and consts
> found in the alias's module or put in as resolved (R7, `Fixed`); the
> nodes are new, so what's kept by node is kept for that use alone, as
> for a copy written by hand.

With A5, A1 of docs/25 holds of refinements too: every check sees the
program with its aliases written out, which each check's own argument is
about already. R9 only refuses more.

Writing the copy out found a bug that was older than refinement aliases:
a refinement in an alias's argument was read at the position it was
written in, not where its parameter is in what the alias stands for. With
`type R<T> = Result<u8, T>`, `R<{ v: u8 | v < 3 }>` refined the `Ok`, not
the `Err`, everywhere alike, so nothing went wrong but the meaning;
`type Pair<T> = (T, T)` couldn't take one at all. Both are written out
now.

## Variants' fields

A variant's field can have a refinement, as a struct's field can, and
its predicate names the fields before it, by their binders: positional
fields have no other names, and a variant's fields are then what a
function's parameters are, each in scope for the ones after it (a
*telescope*, a dependent tuple). A field without a refinement of its
whole value has no binder, so it can't be named.

```rust
#[derive(Copy, Discard)]
enum State {
    Start,
    Utf8(
        { left: u8 | left >= 1 && left <= 3 },
        { c: u32 | (left == 1 && c < 0x8000) || (left == 2 && c < 0x200) || (left == 3 && c < 8) },
    ),
}

fn step(s: State, b: u8) -> (State, Option<u32>) {
    match s {
        State::Start => (State::Utf8(2, (b & 0x0F) as u32), None),
        State::Utf8(left, code) => {
            // No overflow: `code` is below `0x8000`.
            let code = code * 64 + (b & 0x3F) as u32;
            match left > 1 {
                true => (State::Utf8(left - 1, code), None),
                false => (State::Start, Some(code)),
            }
        },
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (s, _) = step(State::Start, 0xE2);
    let (s, _) = step(s, 0x82);
    let (_, c) = step(s, 0xAC);
    out.line(&c.unwrap_or(0))?; // expect: 8364
    Ok(())
}
```

Each is a rule every value of the enum keeps, an invariant as a struct
field's is: making a variant proves its fields' refinements, and a
`match` learns them.

- **V1. Making one proves them** (`Walk::need_variant`), of the values
  given, in order.
- **V2. Every value has them**: of any value of the enum the walk knows
  only its type of, it knows each variant's fields' refinements, if it's
  that variant (`Walk::assume_variants`).
- **V3. A `&mut` into a field is refused where the reference can't say
  what it must keep**: a field whose refinement names another field, or
  that another's names (`Walk::variant_constrained`), or anything in one.
  Into a field whose refinement is about it alone, the reference says it
  (`Walk::own_tree` in `variant_decl`), and what's written through it must
  have it.

> **Lemma E3 (a variant keeps its fields' refinements).** Every value of an
> enum, whenever it can be read, has the refinements of the fields of the
> variant it is. A value is made by its variant (V1: no variant is a
> function value, so each is made where it's named, and proved there), or
> is another moved or copied, which has them: generic code can't make one
> (parametricity). A value changes by a write of a whole value, which has
> them, or of a field of its variant, or a part of one. A field is reached
> only by a pattern, which binds it by value, changing nothing, or by
> `&mut`. If its refinement is about it alone, the reference says it, so
> every write through it keeps it (lemma W of docs/63-what-is-declared.md),
> and a write to a part of it keeps it as the whole of the field
> (lemma V2 of docs/66-one-walk.md). Otherwise the binding is refused (V3).
> So V2 assumes only what holds.

The interpreter checks it: each variant's fields when it's made, and
when a field of it is read.

## A field refined alone

A struct's field whose refinement names no other field, and that no
other field's refinement names, is now the same: a `&mut` to it says its
refinement, so what's written through it must have it, where step 08.1
refused the borrow but straight to a call.

```rust
#[derive(Copy, Discard)]
struct S { a: { v: u8 | v < 3 }, b: u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = S { a: 1, b: 7 };
    let r = &mut s.a;
    *r = 2;
    out.line(&(s.a + s.b))?; // expect: 9
    Ok(())
}
```

> **Lemma F3 (a field refined alone is a declared place).** Its refinement
> is what's declared of its place (`field_decl`), as a refined `&mut`'s
> pointee's is of what it points to: a reference to it says it, and what's
> written through that, by anyone, must have it (lemma W); lent to a call,
> it's proved again after, as before; given to code that can keep the
> reference, what that code may write must have it (step 60's `&mut`
> invariance). Only one field's value is in it, so no write elsewhere
> breaks it. A field another refinement is about still can't be: what's
> written to it would have to keep a fact about another field's value,
> which the reference doesn't know.

## Decisions

- **Checked where declared.** The alternative was checking each use as
  if written there, as bounds in what an alias stands for are, which
  would allow `Pos<i64>` and `T: Int` uses. Checking it once is what a
  generic function's refinements get, and bounds on aliases' parameters,
  once there are, will let a predicate count on them.
- **Const parameters in predicates.** An alias already took const
  parameters; refusing them in predicates would have been a rule with no
  argument behind it.
- **Variants' fields name the fields before them, by binder.** Only a
  field's own value (self-only) wasn't enough for the decoder above: its
  number's bound depends on how many bytes are left. Struct-like variants,
  with named fields, would be another way, and a feature of their own.
- **A field refined alone, of a struct or a variant, is carried by a
  `&mut`.** Variants' fields needed the rule, and a struct's field the
  same shape should be the same.
- **0.81.0.** Programs refused before are accepted, and the alias
  argument fix changes what some accepted ones mean.

## What it doesn't do

- An alias can't take a value: `type Lt<n> = { v: usize | v < n }` for a
  `n` that's a parameter of the function using it. Liquid Haskell's
  aliases can.
- An alias's parameters still have no bounds, so `Pos<T: Int>` waits for
  checked bounds on aliases.
- Messages show what an alias stands for, `s < 64`, not `Square`.
- A variant's field without a refinement can't be named by another's;
  `{ n: u8 | true }` gives it a binder.
- A struct's field refined alone, of a temporary, can't be lent `&mut` to
  a call: there's no place to prove it again of after.

## Exercises

1. Write `type Grid<const W: usize, const H: usize> = { v: List<u8> |
   v.len() == W * H }` and a function indexing it by `Below<W>` and
   `Below<H>`. Which proof needs the product?
2. Turn off `Fixed` and write two programs where a name in an alias, or
   one given to it, is caught by another: one with a const parameter, one
   with a const.
3. Why can't a `&mut` be bound to the `left` of `State::Utf8` above?
   Write the program that would go wrong.
4. Lemma E3 says no variant is a function value. What would have to be
   checked if `State::Utf8` could be passed to `map`?

## Reading

- Vazou, Seidel, Jhala, Vytiniotis and Peyton Jones, *Refinement Types
  for Haskell* (ICFP 2014): Liquid Haskell's refinement aliases (`type
  Pos = {v:Int | 0 < v}`, with value parameters) and refined data
  constructors whose fields name the fields before them.
- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust* (PLDI
  2023): refined type aliases with index parameters, and refined enums.
- Xi and Pfenning, *Dependent Types in Practical Programming* (POPL
  1999): datatypes indexed by integers, the other way to say what a
  variant's fields are.
