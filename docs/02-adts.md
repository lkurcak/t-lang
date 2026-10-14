# Step 2: algebraic data types and pattern matching

Code: `src/check.rs` (typing rules), `src/exhaust.rs` (exhaustiveness), and
`src/interp.rs` (what matching does at runtime).

```rust
enum Shape { Circle(int), Rect(int, int), Empty }

fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}
```

## Two ways to build a type

Step 1 had a fixed set of types. Step 2 lets programs define their own, and
there are exactly two basic ways to do it:

- **Product** (`struct`): a value has field 1 **and** field 2 **and** ...
  `struct Point { x: int, y: int }` holds both an `x` and a `y`.
- **Sum** (`enum`): a value is variant 1 **or** variant 2 **or** ...
  A `Shape` is a `Circle` or a `Rect` or `Empty`, never more than one, and it
  carries only that variant's fields.

C has both too, as `struct` and `union`, but a C `union` doesn't remember
which variant it holds. You track that yourself with a separate tag field, and
nothing stops you from reading the wrong variant. An `enum` here is a *tagged*
union where the tag is checked: the only way to get at the fields is a `match`
on the tag.

## Why "algebraic"

Count the values of each type:

| Type | Values | Count |
|---|---|---|
| `enum Never {}` | none | 0 |
| `()` | `()` | 1 |
| `bool` | `true`, `false` | 2 |
| `struct P { a: bool, b: bool }` | `P { a: true, b: true }`, ... | 2 × 2 = 4 |
| `enum E { A(bool), B(()) }` | `E::A(true)`, `E::A(false)`, `E::B(())` | 2 + 1 = 3 |

Structs multiply and enums add, which is where the names *product* and *sum*
come from. The usual algebra laws hold too, up to renaming the values:
`struct { a: A, b: B }` ≅ `struct { b: B, a: A }` (commutativity),
`enum { X(A, ()) }` ≅ `A` (× 1), and a struct with a `Never` field has
0 values, just as `n × 0 = 0`.

## Introduction and elimination

Every type has rules for making values (**introduction**) and rules for using
them (**elimination**). Each pair is written above its code in `check.rs`:

```text
  Γ ⊢ e1 : T1 .. Γ ⊢ en : Tn                      Γ ⊢ e : S   S has field f : T
  ---------------------------------- (T-Struct)   ---------------------------- (T-Field)
  Γ ⊢ S { f1: e1, .., fn: en } : S                         Γ ⊢ e.f : T
```

A product is made from all its parts and used by taking one out. A sum is the
other way around: you make it from *one* part, and using it means being ready
for *any* part:

```text
  enum E { .., V(T1..Tn), .. }   Γ ⊢ ei : Ti
  ------------------------------------------ (T-Variant)
            Γ ⊢ E::V(e1..en) : E

  Γ ⊢ e : T    pi : T ⊣ Δi    Γ, Δi ⊢ ei : R    p1..pn cover T
  ------------------------------------------------------------ (T-Match)
           Γ ⊢ match e { p1 => e1, .., pn => en } : R
```

`p : T ⊣ Δ` is a judgment about patterns: "`p` matches values of type `T` and
binds the variables `Δ`". For example, `Shape::Rect(w, _) : Shape ⊣ w : int`.
That's `Checker::pattern`. Each arm's body is checked in a context extended
with that arm's bindings, just as T-Let extends the context with one variable.

An or-pattern `p | q` binds the same Δ as each of its alternatives, so they
must all agree on it, with the same variables at the same types:

```text
  p : T ⊣ Δ    q : T ⊣ Δ
  ---------------------- (P-Or)
      p | q : T ⊣ Δ
```

`Shape::Circle(r) | Shape::Rect(r, _)` is fine. `Opt::Some(x) | Opt::None`
isn't: when the value is `None`, the body would read an `x` that was never
set. That's a stuck state, so this rule matters for soundness too.

## Exhaustiveness is part of soundness

The last premise of T-Match, "p1..pn cover T", says every value of type `T`
matches at least one pattern. Without it:

```rust
match s { Shape::Circle(r) => r }   // and s is Shape::Empty?
```

the program would be **stuck**, with no rule saying what happens next. That's
exactly what step 1's soundness theorem rules out. So exhaustiveness checking
isn't a lint, it's how *progress* survives the addition of sum types. In
`interp.rs`, a `match` where no arm fits calls `unsound(..)`, next to adding a
bool and calling an int.

The same goes for `let`: `let Shape::Circle(r) = s;` is rejected because `s`
might be something else (`tests/t/err_let_refutable.t`). Patterns that can't
fail, like `let Point { x, y } = p;`, are fine.

## Checking exhaustiveness: usefulness

`src/exhaust.rs` uses the algorithm from Maranget's *Warnings for pattern
matching* (2007), which is also the basis of rustc's and OCaml's checkers.
It answers one question:

> Given rows of patterns, is there a value that pattern `q` matches but no row
> does? If so, `q` is **useful**, and that value is the **witness**.

Both checks are then one-liners:

- **Exhaustive:** `_` is *not* useful after all the arms. If it is, the
  witness is the error message: ``non-exhaustive `match`: `Shape::Rect(_, _)`
  not covered``.
- **Unreachable arm:** arm *i*'s pattern is not useful after arms `0..i`.

It works on a matrix (one row per arm, one column per value still to be
examined), peeling off the first column each time. Values are seen by their
**constructor**: `true`, `Shape::Rect`, `Point`. For `q`'s first pattern:

1. **A constructor `c`:** only rows that could start with `c` matter.
   *Specialize*: keep those rows, replace their first pattern with `c`'s field
   patterns (`_` becomes `_, _` for a 2-field `c`), and recurse.
2. **`_`, and the rows mention every constructor of the type:** `_` is useful
   if and only if one of those constructors is, so try each with case 1.
   Constructors that can't hold a value (see *The empty type*) don't need to
   be mentioned.
3. **`_`, and some constructor is never mentioned:** a value built with it
   gets past every row that has a constructor there. Only rows starting with
   `_` can stop it, so drop the rest (the *default matrix*) and recurse on the
   remaining columns. The witness names the unmentioned constructor.

When no columns are left, `q` is useful exactly when no rows are left.

**Or-patterns** fit in without a new case. A row starting with `a | b` is
split into two rows, one starting with `a` and one with `b`, since it stops
exactly the values either of those would.

Unreachable *alternatives* need more care than unreachable arms.
`Opt::Some(1 | 1)` hides its duplicate inside a variant. In

```rust
Opt::Some(1) => ..,
Opt::Some(1 | 2) | Opt::None => ..,
```

the inner `1` is dead because of the arm above, but the second arm as a whole
is fine, thanks to `2` and `None`. So `exhaust::redundant` expands each arm
into every combination of its alternatives, in the order they're tried
(`Some(1)`, `Some(2)`, `None`), and checks each one against all the arms and
combinations before it. An alternative is unreachable if no combination that
uses it is useful, and an arm is unreachable if none of its combinations are.

Worked example, with `struct Pixel { color: Color, bright: bool }` and
`enum Color { Red, Green }` (`tests/t/err_match_missing_nested.t`):

```text
Pixel { color: Red,   .. }            is `_` useful?
Pixel { color: Green, bright: true }

Pixel is the only constructor and both rows use it (case 2),
so specialize by Pixel. Columns are now (color, bright):

  [Red,   _   ]      q = [_, _]
  [Green, true]

color: both Red and Green are mentioned (case 2). Try each:
  Red:   rows [_]     q = [_]   bright: nothing mentioned (case 3), default
                                matrix = [[]]: a row remains, so not useful.
  Green: rows [true]  q = [_]   bright: `false` is never mentioned (case 3),
                                default matrix = []: useful, witness `false`.

Reassemble: Pixel { color: Green, bright: false } is not covered.
```

`int` has too many constructors to list (one per number), so a match on an
int is always case 3 and needs a `_` or a variable somewhere. Function values
have no constructors patterns can name at all.

Usefulness is NP-complete in general: you can encode SAT as a match on
`bool`s. Real matches are small and shallow, so in practice it's fast.

## The empty type

`enum Never {}` has no variants and therefore no values. A function taking a
`Never` can never be called. A `match` on one needs no arms (case 2 with zero
constructors: `_` is never useful), and since that match can never run, it
can have *any* type:

```rust
fn absurd(n: Never) -> int { match n {} }
```

In logic this is "from a contradiction, anything follows". Rust's `!` and
`std::convert::Infallible` are this type.

The same goes for a variant that holds a `Never`: no value can be built with
it, so it needs no arm, and an arm for it is unreachable
(`tests/t/empty_enum.t`, `err_match_uninhabited_arm.t`). `exhaust.rs` asks
whether a type is **inhabited** (has any values) to know this. It has to ask
in case 2 and not just leave it to the recursion. Otherwise the two checks
disagree: reachability finds the `Impossible(n)` arm useless (specializing
reaches a `Never` column with nothing to try), while exhaustiveness demands it
(case 3 sees an unmentioned constructor), and no program would pass both.

The empty match needed a small change to the checker too. In step 1, checking `e` against a
type was always "synthesize `e`'s type, compare". An empty match has no arm to
synthesize a type from, so blocks and `match` now push the *expected* type down
to their results instead (`Checker::check`). This is a first glimpse of
bidirectional typing, which is step 3. Without an expected type, say in
`let x = match n {};`, it's an error.

## Design decisions worth noticing

- **Nominal typing.** `struct Meters { v: int }` and `struct Feet { v: int }`
  are different types, though they have the same shape (`Type::Adt` holds a
  *name*). That's what makes them useful for keeping units apart. Structural
  typing (TypeScript, OCaml objects) would compare the fields instead.
- **Variants are always qualified:** `Shape::Empty`, not `Empty`. In Rust,
  `match x { Empty => .. }` with `Empty` *not* in scope silently binds a new
  variable named `Empty` and matches everything. Requiring the prefix makes
  that mistake impossible here.
- **Unreachable arms and alternatives are errors**, not warnings: T has no
  warnings yet. They don't break soundness, but they're almost always a bug.
- **No struct literals in `if`/`match` heads.** In `if x { .. }`, is `x {`
  a struct literal? Rust's answer, copied here: not in that position. Use
  parentheses: `if p == (P { x: 1 }) { .. }` (`tests/t/if_struct_literal.t`).
- **No recursive types.** `enum List { Nil, Cons(int, List) }` is rejected:
  a `List` would contain a `List`, so it would have infinite size. C needs a
  pointer for this, and so do we, which means deciding who frees it (no GC!)
  and how the type checker reasons about a type that refers to itself.
  That's step 5.
- **`==` on structs and enums** compares field by field, unless a function
  is in there somewhere (same reason as step 1's T-Eq).

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `enum Option<T>` | generic types | 4 |
| `enum List { Nil, Cons(int, List) }` | recursive types | 5 |
| tuples `(int, bool)` | anonymous structs; easy, just not needed yet | — |
| guards `x if x > 0 => ..` | they make exhaustiveness approximate: the checker can't see through `if` | — |
| mutation of fields | effects, ownership | 7 |

## Exercises

1. Write a match that `exhaust.rs` reports as non-exhaustive with a witness
   at least three constructors deep. Then trace the algorithm by hand.
2. Delete the exhaustiveness check in `Checker::match_expr` and write a
   program that hits `unsound` in the interpreter. Compare with step 1's
   exercise 2.
3. Expanding every combination of alternatives is exponential: write one arm
   that `exhaust::redundant` expands into 1024 rows. Can you find unreachable
   alternatives without expanding? (Hint: `useful` already accepts rows that
   contain or-patterns and splits them only when they reach the first
   column.)
4. How many values does `enum E { A(bool, bool), B(Never), C }` have? Write
   a match with one arm per value and no wildcards, and check it's accepted.
   Then remove the `inhabited_ctor` filter in `useful` and see which of your
   arms the checker now asks for, and which one it calls unreachable when
   you add it.
5. `Cx::inhabited` assumes types aren't recursive. Find a recursive type
   that has no values, which step 5 will have to handle.

## Reading

- *Types and Programming Languages* (Pierce), chapter 11 (sums, variants,
  records).
- Luc Maranget, *Warnings for pattern matching*, Journal of Functional
  Programming, 2007. Section 3 is the algorithm in `exhaust.rs`.
- The rustc dev guide's chapter on pattern and exhaustiveness checking, for
  what a production version adds (ranges, slices, references, privacy).
