# Step 58: tuples

Code: `Type::Tuple` and `struct_fields` in `src/types.rs`; `parens` and
the field index in `postfix` in `src/parser.rs`; T-Tuple in
`src/check/expr.rs`, tuple patterns in `src/check/patterns.rs`; the
built-in rule in `Traits::implements` in `src/traits.rs`; the closure
parameters' rewrite, `Rewrite::param_patterns`, in
`src/check/modules.rs`; `{cmp}` and the tuple cases of `{eq}`, `{drop}`
and `{debug}` in `src/mono/glue.rs`; `in_refined_local` and
`keep_invariant` in `src/check/refine.rs`; `enumerate` and `zip` in
`core/iter.t`.

A function that gives two things back has had to declare a struct for
them. Core had four that existed for nothing else: `split_at` gave a
`Split { head, tail }`, `split_at_mut` a `SplitMut`, a string's
`split_at` a `StrSplit`, and an environment's variables were each a
`Var { name, value }`. Programs wrote tuples by habit, and rewrote them:
a nonogram's clue range as `(lo, hi)`, two arguments that both had to
parse as `let (Some(w), Some(h)) = (w, h) else { .. }`.

A tuple is two or more values side by side, of any types:

```rust
fn div_rem(a: u32, b: { b: u32 | b > 0 }) -> (u32, u32) { (a / b, a % b) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (q, r) = div_rem(17, 5);
    out.line(&q)?; // expect: 3
    out.line(&r)?; // expect: 2
    let p = div_rem(9, 4);
    out.line(&(p.0 * 4 + p.1))?; // expect: 9
    Ok(())
}
```

## Tuples

`(A, B)` is the type of pairs of an `A` and a `B`, `(a, b)` makes one,
and `p.0` and `p.1` are its fields, which a pattern `(x, y)` takes
apart. Three or more are the same: `(1u8, 'c', true)`. Fields nest, and
a field of a `mut` variable can be written, moved out of and borrowed on
its own, as a struct's can:

```rust
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut n: ((u8, u8), (u8, (u8, u8))) = ((1, 2), (3, (4, 5)));
    n.1.1.0 = 40;
    out.line(&n.1.1.0)?; // expect: 40
    let mut t = (String::from("ab"), 3u8);
    let s = t.0;
    t.0 = String::from("xyz");
    out.line(&(s.len() + t.0.len()))?; // expect: 5
    let a = &mut t.1;
    *a += 1;
    out.line(&t.1)?; // expect: 4
    Ok(())
}
```

`()` is the tuple with none. There are no tuples of one: `(x,)` would be
`x` in other clothes, and `(x)` only groups, as it did.

```text
let _t = (1u8,);
// error: a tuple has no elements or at least two; for one value, leave out the parentheses
```

Patterns take tuples apart in a `let`, a `match`, a `for` and a closure's
parameters, through references as they do structs. A `match` on a tuple
of values is how to look at several at once:

```rust
fn both(a: Option<u8>, b: Option<u8>) -> u8 {
    match (a, b) {
        (Some(x), Some(y)) => x + y,
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&both(Some(2), Some(3)))?; // expect: 5
    out.line(&both(None, Some(4)))?;    // expect: 4
    let pairs: [(u8, u8); 3] = [(1, 2), (3, 4), (5, 6)];
    let mut sum: u8 = 0;
    for (a, b) in pairs.iter() {
        sum += *a * *b;
    }
    out.line(&sum)?; // expect: 44
    Ok(())
}
```

## What a tuple is

A tuple is a struct no one declared: `(A, B)` is as if core had

```text
#[derive(Copy, Debug, Discard, Eq, Ord, Send, Sync)]
export struct Tuple2<A, B> { export 0: A, export 1: B }
```

with one such struct for every number of fields, and every rule treats
it as that struct. Its fields are exported, so every module can make,
read and take one apart. Each derive needs of `A` and `B` only that
they're the same, so a tuple is `Copy`, `Discard`, `Eq`, `Ord`, `Debug`,
`Send` or `Sync` exactly when all its values are. It compares field by
field, the first that differs deciding, and shows its values in
parentheses:

```rust
use alloc::string::String;

struct Token { id: u8 }

fn spend(t: Token) -> u8 {
    let Token { id } = t;
    id
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&((1u8, 'b') < (2u8, 'a')))?; // expect: true
    let t = (1u8, "two", (true, '3'));
    out.line(&f"{t:?}")?; // expect: (1, "two", (true, '3'))
    // A tuple holding a `Token`, which isn't `Discard`, isn't either: it
    // must be taken apart, and the `Token` used up.
    let (tok, s) = (Token { id: 4 }, String::from("x"));
    out.line(&(spend(tok) as usize + s.len()))?; // expect: 5
    Ok(())
}
```

The difference from a struct is the name. Two struct types are equal
only if they're the same declaration (docs/02); two tuple types are
equal if their values' types are, in order: `(u8, bool)` is
`(u8, bool)` wherever it's written. That's the same thing said of the
encoding: there's one `TupleN` for each `N`, so equal arguments make
equal types.

Tuples are core's, as `i64` is: a package can implement its own trait
for them, and nothing else can implement core's.

```rust,error
use core::fmt::{Display, Write};

impl<A, B> Display for (A, B) {
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("pair") }
}

fn main() {}
// error: `Display` and `(A, B)` are both `core`'s, so only `core` can implement one for the other
```

## In refinements

A tuple's fields are fields, so a predicate names them as it names a
struct's (docs/56): `{ p: (usize, usize) | p.0 < p.1 }`. What's known of
the values a tuple is made of is known of its fields, and a pattern
binds what's known of each:

```rust
fn split(n: { n: u8 | n >= 2 }) -> { p: (u8, u8) | p.0 + p.1 == n && p.0 >= 1 } { (1, n - 1) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [10, 20, 30, 40];
    let i: usize = 2;
    let p = (i, i + 1);
    out.line(&a[p.1])?; // expect: 40
    let (j, k) = p;
    out.line(&a[k - j])?; // expect: 20
    let rest: { r: u8 | r <= 4 } = split(5).1;
    out.line(&rest)?; // expect: 4
    Ok(())
}
```

`split_at` says how long its halves are by their fields:
`Option<{ s: (&[T], &[T]) | s.0.len() == mid && s.1.len() + mid ==
self.len() }>`, and a string's `split_at` the same of a `(&str, &str)`.

## `#[derive(Ord)]`

Tuples compare in order, and now so can structs and enums: `Ord` can be
derived. A struct compares field by field, in the order they're
declared; an enum by its variants' order first, then the fields of the
one both are. Generic structs need `Ord` of what their fields need it
of, as other derives do. `trait Ord: Eq`, so a derived `Ord` needs an
`Eq`, derived or written:

```rust
#[derive(Copy, Discard, Eq, Ord, Debug)]
struct Version { major: u8, minor: u8 }

#[derive(Copy, Discard, Eq, Ord)]
enum Rank { Low, Mid(u8), High(u8, bool) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let v = Version { major: 1, minor: 9 };
    out.line(&f"{core::cmp::max(v, Version { major: 2, minor: 0 }):?}")?; // expect: Version { major: 2, minor: 0 }
    out.line(&(Rank::Low < Rank::Mid(0)))?;              // expect: true
    out.line(&(Rank::Mid(5) < Rank::Mid(4)))?;           // expect: false
    out.line(&(Rank::High(0, false) > Rank::Mid(200)))?; // expect: true
    Ok(())
}
```

```rust,error
#[derive(Copy, Discard, Ord)]
struct V { n: u8 }

fn main() {}
// error: `V` doesn't implement `Eq`, which `Ord` needs, as it's `trait Ord: Eq`
```

## Patterns as closure parameters

A closure's parameter can be a pattern that matches every value: a
tuple's, a struct's or an array's. `|(i, x)| body` is `|p| { let (i, x)
= p; body }`, and binds as that `let` would, by reference through a
reference:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let add = |(a, b): (i64, i64)| a + b;
    out.line(&add((2, 3)))?; // expect: 5
    let pairs: [(u8, char); 3] = [(1, 'a'), (2, 'b'), (3, 'c')];
    let big = pairs.iter().filter(|(n, _)| *n >= 2).count();
    out.line(&big)?; // expect: 2
    Ok(())
}
```

A function's parameters are still names: its refinements and `ensures`
name them, and a caller's errors do.

## In core and std

- `Iterator::enumerate` pairs each value with its index, from 0, and
  `Iterator::zip` a value of each of two iterators, until either runs
  out. A value of the first with none of the second to go with it is
  thrown away, so `zip` needs those to be `Discard`. Counting past the
  greatest `usize` panics, as `count` does.
- `split_at` and `split_at_mut` of a slice, and `split_at` of a `str`,
  give a tuple of the halves. `core::slice::Split`, `SplitMut` and
  `core::str::StrSplit` are gone.
- `Env::vars` gives each variable as `(name, value)`; `std::env::Var` is
  gone.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let words = ["ab", "cde", "f"];
    for (i, w) in words.iter().enumerate() {
        out.line(&f"{i}: {w}")?;
        // expect: 0: ab
        // expect: 1: cde
        // expect: 2: f
    }
    let xs: [u8; 3] = [1, 2, 3];
    let ys: [u8; 4] = [10, 20, 30, 40];
    let mut dot: u32 = 0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        dot += *x as u32 * *y as u32;
    }
    out.line(&dot)?; // expect: 140
    Ok(())
}
```

## A hole closed: writing part of a refined variable

A refined `let mut`'s refinement is an invariant (docs/08): every
assignment to it is checked, and it can't be borrowed `&mut`. But a
write to a *field* of it went unchecked, as did a `&mut` of a field, so
a refinement about its fields could stop holding while the program went
on trusting it. Since refinements of any type (docs/21) this was
reachable with structs; `t.0 = ..` makes it easy to write. The
interpreter caught it, as a broken refinement of an argument:

```rust,error
#[derive(Copy, Discard)]
struct P { a: usize, b: usize }

fn get(p: { p: P | p.a < 3 }) -> usize { p.a }

fn main() {
    let mut p: { p: P | p.a < 3 } = P { a: 1, b: 2 };
    p.a = 7;
    let _ = get(p);
}
// error: can't prove `p.a < 3` for `p`
```

Now a write to any part of a refined local, through fields and boxes,
proves its refinement again of the whole value as it is after, and no
part of one can be lent `&mut`. A write that keeps it is fine:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut t: { t: (u8, u8) | t.0 <= t.1 } = (0, 9);
    t.0 = t.1;
    out.line(&t.0)?; // expect: 9
    Ok(())
}
```

## Why it's sound

- **Lemma T1 (encoding).** Every judgment about a tuple type
  `(A1, .., An)`, `n ≥ 2`, is the one about `TupleN<A1, .., An>`, the
  struct above: a literal is its struct literal (T-Tuple), `t.i` is
  field `i` (T-Field), a pattern is its struct pattern mentioning every
  field, in order, so exhaustiveness sees one constructor; moves, borrows
  and drops go field by field, as `struct_fields` gives them to the
  borrow checker, drop elaboration, the refinement walk and
  monomorphisation alike. Two tuple types are equal exactly when the
  encodings are: there's one struct per arity, so equal arities and
  equal arguments. So every lemma proved of structs holds of tuples:
  preservation and progress, the borrow checker's, drop elaboration's
  (docs/32.1), the field paths' (docs/56).
- **Lemma T2 (kinding).** Each value of a tuple has a size: its types
  are resolved where values are (lemma U1 of docs/57), so no `str`,
  slice or `unsized` parameter is one. A tuple of types of kind ★ is of
  kind ★, which substitution keeps (U2).
- **Lemma T3 (variance).** A tuple is covariant in each of its types,
  as a struct is in a parameter that's only its fields' whole types:
  a field is owned, not behind a `&mut` or a function's argument. Behind
  a `&mut`, the reference makes it invariant, as it does a struct
  (`err_soundness_tuple_invariant.t`).
- **Lemma T4 (traits).** A derive needs, of each field's type, the
  trait derived (docs/17, docs/43.1); a tuple's fields' types are
  exactly its parameters, so the least fixed point is "each `Ai`
  implements it". That's the built-in rule (`Traits::implements`), for
  `Copy`, `Discard`, `Eq`, `Ord`, `Debug`, `Send` and `Sync`, and it
  agrees with what the glue does: `{eq}`, `{cmp}`, `{debug}` and
  `{drop}` go field by field, each by its type's impl. `Copy` needs
  `Discard`: each `Ai: Copy` gives `Ai: Discard`, so the tuple's holds.
  `Ord` needs `Eq` likewise. The rule can't overlap an impl: only core
  could write one for a tuple of a core trait (the orphan rule), and it
  writes none. A proof that a tuple implements a trait is the proofs for
  its values, so the check against polymorphic recursion (docs/29)
  follows it to their impls: a `cmp` that compares a tuple holding its
  own type at a bigger one is caught (`err_soundness_tuple_poly_rec.t`).
- **Lemma T5 (refinements).** To the logic a tuple's value is a
  struct's, its fields' values, each a function of the value (lemma M1
  of docs/21): `p.0` is a field path. Every field is exported, so a
  predicate can name it wherever the tuple is seen.
- **Lemma T6 (consts).** Plain data (docs/36) is closed under tuples: a
  tuple of plain data is a tree of literals the interpreter can give
  back and a use can make again.
- **Lemma T7 (layout).** In C a tuple is a struct of its fields, `f0`,
  `f1`, .., in order, one per tuple type monomorphisation makes, as for
  a struct; its layout is the C compiler's, unspecified to T programs
  (docs/33).
- **Lemma O1 (derived `Ord`).** Nothing relies on `Ord`'s laws to be
  sound (docs/06e): a wrong order gives wrong answers, not undefined
  behavior. `{cmp}` reads each field by `&` and calls its type's `cmp`,
  which is total and effect-free, as the trait's signature says; so it
  is too. A derived `Ord` needs an `Eq` (`trait Ord: Eq`), checked as a
  written impl's supertraits are: otherwise generic code over `T: Ord`
  could call an `==` that monomorphisation can't find. And each derived
  `cmp` is a node of the graph the check against polymorphic recursion
  walks, as each derived `eq` and `fmt` is, with an edge to each field's
  `cmp`: without it, a `cmp` comparing a derived struct that holds its
  own type at a bigger one would make copies without end
  (`err_soundness_derive_ord_poly_rec.t`).
- **Lemma P1 (closure parameters).** A pattern parameter is a fresh
  name the body's first `let` takes apart, so it's checked as that `let`
  is: it must match every value (exhaustiveness, else "refutable pattern
  in closure parameter"), it binds by reference through references, and
  what it doesn't bind is dropped, which needs `Discard`
  (`err_soundness_tuple_linear_closure_param.t`).
- **Lemma R (refined locals).** A refined local's refinement holds of
  its value wherever the walk assumes it: at its `let`, at each
  assignment to it, at each write to a part of it (fields, and what a
  box points to: its value's parts), each of which proves it of the
  whole value after the write; and nothing else changes it, as no part
  of it is lent `&mut` and what's behind a reference isn't part of it
  (no `mut` variable's refinement names what's behind a `&mut`, docs/56).

The adversarial tests are `tests/t/err_soundness_tuple_*.t`,
`err_soundness_derive_ord_bound.t` and
`err_soundness_refined_local_field_*.t`.

## What it doesn't do

- No refinements inside a tuple's types, `(u8, { v: u8 | v < n })`, as
  an enum's type arguments can have (docs/53). A tuple's fields are
  whole and exported, so lemma E2 would carry over; deferred until a
  program needs it.
- No tuple structs, `struct Meters(u64)`, and no 1-tuples.
- No `..` in tuple patterns: `(a, ..)` would need to say how many it
  skips; write `_` for each.
- No patterns as a function's parameters, only a closure's.
- No `Display` for tuples, as in Rust: how to show one depends on what
  it's for.
- Tuples don't coerce: `&(A, B)` isn't `(&A, &B)`; a pattern through the
  reference gives that.

## Decisions

- D1, no 1-tuples: `()` and two or more. `(T,)` adds nothing over `T`;
  Rust has it for macros and impls written per arity, which T has
  neither of.
- D2, `Ord` by a general `#[derive(Ord)]`, as glue, rather than impls in
  core per arity: tuples of any length get it as they get `Eq` and
  `Debug`, and structs and enums can derive it too.
- D3, core's pair structs become tuples, and `enumerate` and `zip` are
  new. `str::Found { start, end }` stays named: its fields say more than
  `.0` and `.1` would.
- D4, closures take patterns as parameters; functions still take names.
- D5, refined tuple element types: deferred.
- And: a tuple is a type of its own, `Type::Tuple`, compared by what's
  in it, rather than a struct in core per arity. It has no limit on its
  length, and needs no definitions; every phase reaches its fields
  through `struct_fields`, which gives a tuple's as a struct's, so it
  has the encoding's rules without one being written.

## Exercises

1. Write `fn min_max(xs: &[i64]) -> Option<(i64, i64)>`, and use it with
   `let Some((lo, hi)) = min_max(..) else { .. }`.
2. Why does `zip` need its first iterator's values to be `Discard`, but
   not its second's? Write a `zip` for which the reverse holds.
3. A tuple `(A, B)` is `Ord` if `A` and `B` are. Why isn't it enough
   that `A` is, when two tuples' first values are never equal?
4. Take `keep_invariant` out of the assignment of a field. Which test
   then runs, and what does the interpreter say?
5. `(T,)` isn't a type. Write the `impl` of a trait for tuples of every
   length that Rust would need it for, and explain why T doesn't.

## Reading

- Pierce, *Types and Programming Languages* (2002), chapter 11: pairs,
  tuples and records; chapter 15: subtyping, and records' width and
  depth.
- Milner, Tofte, Harper and MacQueen, *The Definition of Standard ML
  (Revised)* (1997): a tuple `(e1, .., en)` is the record
  `{1 = e1, .., n = en}`, a derived form, which is T's encoding with
  fields `0` to `n - 1`.
- Harper, *Practical Foundations for Programming Languages* (2nd ed.,
  2016), chapter 10: finite products, and the unit type as the empty
  one.
- The Haskell 2010 Report, section 6.1.4: tuples as algebraic data
  types, with instances derived for each arity.
