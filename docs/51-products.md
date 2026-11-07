# Step 51: products of two unknowns

Code: `Kind::Product`, `times` and `lemmas` in `src/check/refine/logic.rs`;
`Walk::quotient`, `degree` and `Walk::read_obj` in `src/check/refine.rs`;
`checked_mul` in `core/num.t`; `List::filled`, `replace` and `set` in
`alloc/list.t`.

Step 8 kept the refinement logic linear: sums of names times constants. A
grid stored row by row has its cell at `y * width + x`, a product of two
unknowns, so that index could never be proved in range, and the grid's
invariant, `cells.len() == width * height`, couldn't even be written.
t-nonograms paid for it on every cell: each read went through an `Option`
that couldn't be `None`.

Now a product of two unknowns is a value of the logic, and the checker
knows enough about it to prove the grid:

```rust
use alloc::list::List;

#[derive(Discard)]
struct Grid { width: usize, height: usize, cells: { c: List<u8> | c.len() == width * height } }

impl Grid {
    #[measure]
    fn width(&self) -> usize { self.width }

    #[measure]
    fn height(&self) -> usize { self.height }

    fn get(&self, x: { x: usize | x < self.width() }, y: { y: usize | y < self.height() }) -> u8 {
        self.cells[y * self.width + x]
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let g = Grid { width: 2, height: 1, cells: List::filled(2, 5) };
    if g.width() > 1 && g.height() > 0 {
        out.line(&g.get(1, 0))?; // expect: 5
    }
    Ok(())
}
```

## What the logic knows of a product

`u * v` is a new variable of the logic, one for each pair, so `w * h` and
`h * w` are the same. Multiplying sums multiplies out: `(y + 1) * w` is
`y * w + w`. A product of three unknowns isn't in the logic; in a
refinement it's an error:

```rust,error
fn f(k: { v: i64 | v * v * v < 10 }) -> i64 { k }

fn main() {}
// error: a refinement can multiply at most two unknowns: `v * v * v` multiplies three
```

and in code, the operator isn't proved, as any other the logic can't say.

What the checker knows of `u * v` comes from what it knows of `u` and `v`,
by *lemmas*: statements true of every pair of integers, added to a proof
before it's tried. For a bound on one factor, `u <= L`, and the other's
sign:

- if `v >= 0`, then `u * v <= L * v`;
- if `v <= 0`, then `u * v >= L * v`;

and the same for `u >= L` (the other way round), for `u == L` (`u * v ==
L * v`, whatever the sign), with `u` and `v` swapped, and for the
constant bounds of `u`'s type. `L * v` is multiplied out, so a bound
mentioning other names gives products of those. A square is never
negative.

The grid's index is three of these. `y < height` is `y <= height - 1`,
and `width >= 0`, so `y * width <= height * width - width`; with `x <
width`, `y * width + x < height * width`, which is `cells.len()`. That's
the index in range, and `y * width` and the sum fit in a `usize`, being
at most `cells.len()`.

The bounds and signs come from the case being refuted: after the
procedure has split the question on its `||`s (step 8), each case is a
list of plain comparisons, and those are the bounds. A sign is what the
case proves: `v >= 0` if it can't hold with `v <= -1`. When a case says
nothing of a factor's sign, it's split again, into `v >= 0` and `v <= 0`,
for the first three such factors: so `x * x` fits in an `i8` for `-10 <
x < 10`, from `x * x <= 9 * x` on one side and `x * x <= -9 * x` on the
other.

```rust
fn sq(x: { x: i8 | x > -10 && x < 10 }) -> { s: i8 | s >= 0 } with {} { x * x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sq(-9))?; // expect: 81
    Ok(())
}
```

The sign matters. Without it, a bound says nothing:

```rust,error
fn f(
    a: { a: i64 | a > -10 && a <= 5 },
    b: { b: i64 | b > -10 && b < 10 },
) -> { p: i64 | p <= 5 * b } { a * b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, -1))?;
    Ok(())
}
// error: can't prove `p <= 5 * b` for the result of `f`
```

## What division says

Dividing by an unknown now says something too, as a product. `q = a / b`
rounds towards zero, so `a` is `q * b` plus a remainder with `a`'s sign,
smaller than `b`. Where the signs of `a` and `b` are known, that's a
fact: for `a >= 0` and `b > 0`, `q * b <= a <= q * b + b - 1`; the other
three cases are its mirror images. And `q` has the sign of `a * b`.

That's the usual check that a product fits, which proves it does:

```rust
fn times(a: usize, b: { b: usize | b > 0 }) -> Option<usize> with {} {
    if a <= core::num::greatest::<usize>() / b { Some(a * b) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&times(6, 7).unwrap_or(0))?; // expect: 42
    Ok(())
}
```

`a <= q` and `b >= 0` give `a * b <= q * b`, and the division fact gives
`q * b <= greatest`. So `checked_mul` is now written in T, with a case for
each pair of signs, and the checker takes one intrinsic fewer on trust:
of `core::num`, only `least`, `greatest` and `wrapping_mul` are left.

## Lists that are refined fields

The grid needs two things from `List` that it didn't have.

A list of `width * height` copies of a value: `List::filled(n, v)`, whose
result is `n` long. A `while` loop pushing into a local can't say so, as
the checker forgets what it knew of the local at the loop's head and
infers no invariant. `filled` keeps the list in a struct whose refinement
says `len() <= n`, which every iteration keeps.

That needed a fix: a loop that lends a refined field forgot the struct's
refinement too, though it holds of every value the field has. Now a new
value of a field, as after such a loop, has the struct's refinements with
its other fields, as a new value of a scalar field already did.

Writing a cell: `l[i] = v` lends the list for as long as the `&mut T` it
gives lives, which a refined field can't be (step 30). `List::replace(i,
v)` and `List::set(i, v)` lend it for the call only, and say the length
stays: so the grid's `set` keeps its invariant.

```rust
use alloc::list::List;

#[derive(Discard)]
struct Row { width: usize, cells: { c: List<u8> | c.len() == width } }

impl Row {
    #[measure]
    fn width(&self) -> usize { self.width }

    fn set(&mut self, x: { x: usize | x < self.width() }, v: u8)
    ensures
        self.width() == old(self.width()),
    {
        self.cells.set(x, v);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut r = Row { width: 3, cells: List::filled(3, 0) };
    r.set(2, 9);
    out.line(&r.cells[2])?; // expect: 9
    Ok(())
}
```

## Why it's sound

- **Lemmas are theorems.** Each lemma is true of all integers: `u <= L`
  and `v >= 0` give `u * v <= L * v` in ℤ. The checker adds one only when
  its premises are among, or follow from, the comparisons of the case
  it's refuting; the sign split's two sides cover every integer. So any
  assignment of integers satisfying the case satisfies the lemmas too,
  with each product variable at its factors' product, and if the case
  with lemmas has no integer solution, the case alone has none. The
  procedure stays sound and incomplete (step 8): a missing lemma only
  rejects more.
- **A product variable is unconstrained otherwise.** The logic treats
  `u * v` as a variable with no range of its own; anything it concludes
  of it came from a lemma. Interning makes `u * v` and `v * u` one
  variable, which is true of multiplication.
- **Division facts are theorems of truncating division**, used only where
  the signs they need are proved at the division: `a = q·b + r`, `|r| <
  |b|`, `r` with `a`'s sign or 0.
- **It ends.** One round of lemmas per case, from the case's comparisons
  only, never from other lemmas, at most 64 lemmas and 3 sign splits;
  the case and constraint budgets of step 8 still apply. The answer
  depends only on the question.
- **The interpreter still checks.** Every refinement and every operator
  marked proved is checked at run time under `t run --interp`; one that
  fails is "type soundness violated".
- **The field fix** assumes the struct's refinements of a field's new
  value only where the field's place is stable, so the value assumed of
  is the one that's read; and the refinements hold of every value the
  field can have, as every write to it, or lend of it, proves them again.

## Decisions

- Products in the logic, not a grid with an unchecked, `unsafe` index
  inside, nor a grid checked at run time: every grid program gets the
  proof.
- Division facts for divisors that aren't constants, so `checked_mul`
  could be written in T.
- `t-libs`'s `Grid::new` gives `None` when `width * height` doesn't fit,
  rather than panicking.

## Exercises

1. Why isn't `u >= 0 && v >= 0` enough to say `u * v >= u`? What's the
   counterexample, and which lemma does prove `u * v >= u` when `v >= 1`?
2. `y * w + x < w * h` is proved from `y < h` and `x < w`. Prove it with
   pen and paper using only the lemmas above, and count how many the
   checker needs.
3. The lemmas are one round. Find a true statement about a product of two
   unknowns that would need a lemma about a lemma's product.
4. `for x in 0..g.width()` tells the checker nothing about `x` (the range's
   `next` gives an `Option<T>`, unrefined). What would `Range` need to say
   so that `x < g.width()` held in the body?

## Reading

- Dafny's and SPARK's documentation on nonlinear arithmetic: both treat
  products as uninterpreted by default and prove facts about them with
  lemmas the programmer calls.
- Jovanović and de Moura, *Solving Non-Linear Arithmetic* (2012), for
  what a complete procedure does instead.
- Cimatti et al., *Incremental Linearization for Satisfiability and
  Verification Modulo Nonlinear Arithmetic and Transcendental Functions*
  (2018): lemmas on products, added as a solver needs them, which is this
  step's idea taken further.
