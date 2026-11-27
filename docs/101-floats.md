# Step 101: floats

Code: `FloatTy` and `float_bits` (`src/types.rs`); `float_literal`
(`src/lexer.rs`); `Checker::fresh_float`, `float_like` and `float_cast`
(`src/check/unify.rs`, `src/check/expr.rs`), which type floats, their
literals, operators, comparisons and `as`; `not_in_logic`
(`src/check/refine.rs`), which keeps them out of the refinement logic;
`interp::floats`, `float_lit` and `float_cast` (`src/cgen.rs`), and the
shim's `t_f_to_i` and its kin (`src/cgen/shim.h`), which run them; and in
core, `Float` (`core/num.t`), `core/float.t` and `core/float/dec.t`, which
writes them as text and reads them.

T has two floating-point types, `f32` and `f64`: IEEE 754's binary32 and
binary64, the numbers every machine T runs on computes with. Each
operator gives the exact result rounded to the nearest value of the type,
ties to even, and never panics: a result too big is an infinity, and one
there's no answer for, like `0.0 / 0.0`, a NaN, "not a number".

```rust
fn mean(a: f64, b: f64) -> f64 with {} { (a + b) / 2.0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&mean(1.0, 2.0))?;        // expect: 1.5
    out.line(&(0.1 + 0.2f64))?;        // expect: 0.30000000000000004
    out.line(&(1.0f64 / 0.0))?;        // expect: inf
    out.line(&(0.0f64 / 0.0))?;        // expect: NaN
    out.line(&f"{2.0f64.sqrt():.4}")?; // expect: 1.4142
    Ok(())
}
```

`mean` says `with {}`: nothing float arithmetic does is an effect.

## Literals

A literal with a point, `2.5`, or an exponent, `1e-3`, or both, is a
float, of a type worked out from how it's used, as an integer literal's
is: there's no default, so `let x = 1.5;` that nothing says more of is
"cannot infer". A suffix says which, `2.5f32`, and makes digits alone a
float, `1f64`. `1` is never a float, nor `.5`, `1.`, or `1E5`, whose `e`
is written in lowercase. A literal's value is the nearest of its type,
worked out once from the decimal, never through another type: `0.1f32` is
the `f32` nearest a tenth, not the `f64` nearest one, rounded again. And
it must not be an infinity:

```rust,error
fn f() -> f32 { 1e39 }
// error: `1e39` is too big for an `f32`: its nearest value is an infinity

fn main() {}
```

`t.0.1` is still two fields of a tuple, and `1..2` a range: a `.` right
after a `.`, or not followed by a digit, isn't a float's.

## Comparing

`==`, `!=`, `<`, `<=`, `>` and `>=` of floats are IEEE 754's: a NaN is
equal to nothing, itself included, and neither less nor greater than
anything; and `0.0 == -0.0`. So `==` of floats isn't an equivalence, and
`<` no total order, which is what `Eq` and `Ord` promise: a float is
neither. Code over an `Eq` may count on `a == a`, as a hash map does, and
a float can't be given to it:

```rust,error
fn same<T: Eq>(a: &T) -> bool { a == a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&same(&f64::nan()))?;
    Ok(())
}
// error: `same` needs its `T` to implement `Eq`, and `f64` doesn't
```

nor can a struct that holds one derive `Eq`. `total_cmp` orders every
value, NaNs too, as IEEE 754's total order does: negative NaNs, minus
infinity, the negative numbers, `-0.0`, `0.0`, the positive ones,
infinity, positive NaNs.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let nan = f64::nan();
    out.line(&(nan == nan))?;                           // expect: false
    out.line(&(0.0f64 == -0.0))?;                       // expect: true
    out.line(&f"{(-0.0f64).total_cmp(&0.0):?}")?;       // expect: Less
    out.line(&f"{nan.total_cmp(&f64::infinity()):?}")?; // expect: Greater
    Ok(())
}
```

A float can't be a pattern either: a pattern matches what's equal to it.

## Converting

`as` gives the same number, as it does between integer types, or panics
where there's none. Where every value of the type converted is one of the
other's, there's always one, and it can't panic: `i32 as f64`, `u8 as
f32`, `f32 as f64`. From a wider integer, it can't where the refinement
logic proves the integer is at most 2^53 either way (2^24 for an `f32`),
as every such integer is a float. From a float to an integer, or an `f64`
to an `f32`, it may always panic: a float isn't in the logic, so nothing
proves it's an integer in range, or an `f32`.

```rust
/// Every `i32` and `u8` is an `f64`.
fn wide(a: i32, b: u8) -> f64 with {} { a as f64 + b as f64 }

/// A wider integer, where it's said to be in range.
fn exact(n: { n: i64 | n >= -9007199254740992 && n <= 9007199254740992 }) -> f64 with {} {
    n as f64
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&wide(-2, 255))?;             // expect: 253.0
    out.line(&exact(9007199254740992))?;   // expect: 9007199254740992.0
    out.line(&(2.5f64.round() as i32))?;   // expect: 3
    out.line(&f32::nearest(16777217i32))?; // expect: 16777216.0
    out.line(&(9007199254740993i64 as f64))?;
    Ok(())
}
// error: runtime error: `as` failed: 9007199254740993 isn't an `f64` exactly
```

Rounding is said, by name: `F::nearest(n)` is the float nearest integer
`n`, `F::nearest_f64(x)` the one nearest an `f64`, and a float rounds to an
integer by `floor`, `ceil`, `trunc` or `round`, which `as` then converts
exactly.

## `Float`

`Float`, in the prelude, is what `Int` is for the integers: it's sealed,
and `f32` and `f64` are all there is of it, so code over a `F: Float` may
use operators, literals and comparisons, and is written once for both. A
literal of a `F` must be finite in an `f32`, the narrower; `as` of a `F`
must be exact for an `f32` too.

```rust
fn dot<F: Float>(a: &[F], b: &[F]) -> F {
    let mut sum = 0.0;
    let n = if a.len() < b.len() { a.len() } else { b.len() };
    for i in 0..n {
        sum = sum + a[i] * b[i];
    }
    sum
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [f64; 3] = [1.0, 2.0, 3.0];
    let b: [f32; 2] = [0.5, 0.25];
    out.line(&dot(&a, &a))?; // expect: 14.0
    out.line(&dot(&b, &b))?; // expect: 0.3125
    Ok(())
}
```

Its methods: `abs`, `copysign`, `floor`, `ceil`, `trunc`, `round` (halves
away from zero), `sqrt`, `min` and `max`, `is_nan`, `is_finite`,
`is_infinite`, `is_sign_negative`, `total_cmp`, and the values `infinity`,
`nan`, `greatest`, `least_positive` and `epsilon`; each type also has
`to_bits` and `from_bits`. A NaN in gives a NaN out, of `min` and `max`
too, which are IEEE 754-2019's `minimum` and `maximum`: `-0.0` is the less
of the zeros. Which NaN arithmetic makes, its sign and the rest of its
bits, isn't said, as machines differ; `from_bits` keeps every bit.

## As text

`Display` and `Debug` write the fewest digits that read back as the same
float, and of those the nearest, as Rust's `{:?}` does: in decimal, with
at least one digit after the point, from `0.0001` up to, not including,
`1e16`, and in scientific notation past those. `f"{x:.3}"` writes the
exact value, rounded at 3 digits after the point, ties to even, and
`Parse` reads the float nearest the decimal, ties to even:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"{0.1f64} {1e16f64} {1.5e-7f64} {-0.0f64} {1.0f32 / 3.0}")?;
    // expect: 0.1 1e16 1.5e-7 -0.0 0.33333334
    out.line(&f"{0.125f64:.2} {2.675f64:.2} {2.675f64:>8.1}")?;
    // expect: 0.12 2.67      2.7
    match "9007199254740993".parse::<f64>() {
        Ok(x) => out.line(&x)?, // expect: 9007199254740992.0
        Err(e) => out.line(&e)?,
    }
    Ok(())
}
```

All of it is written in T, in `core::float::dec`, on big natural numbers
of 128 limbs on the stack, worked out exactly: the shortest digits by
Burger and Dybvig's algorithm, reading by dividing exactly. So it needs no
allocation, and the checker proves none of it panics; and the interpreter
and the C backend run the same code, so write the same text.

## Why it's sound

- **F1 (values).** A float is its bits, in the interpreter too, so two
  floats are the same value, as the logic and the interpreter's checks of
  `swap` and postconditions see values (docs/85), exactly when their bits
  are: an equivalence, which every function of a float respects.
- **F2 (operators).** IEEE 754's operations are total: each gives a value
  of the type, whatever its operands, so none can panic, and a float
  operation has no effect to prove.
- **F3 (comparisons).** `==` and `<` of floats are the machine's, and no
  law of `Eq` or `Ord` is claimed of them: floats implement neither, and a
  comparison of a type not known when it's checked drops its obligation
  only where the type turns out to be a float. The logic learns nothing
  from one.
- **F4 (not in the logic).** A predicate can't name a float, not even
  whole: so nothing is proved of one, and what the logic knows can't
  depend on what IEEE 754 says of NaN. Generic predicates, over a `T` that
  may be a float, still compare values as F1 says.
- **F5 (`as`).** Converting to a float is proved only where every integer
  in the range it's proved in is a float exactly: each with at most `p`
  bits, `p` the significand's, is. Every other conversion is checked when
  it runs, and panics where there's no same number, before C converts,
  whose conversion of a float out of an integer type's range is
  undefined. The interpreter checks that what's proved holds.
- **F6 (text).** The big numbers have room for the greatest any
  conversion makes: about 1100 bits to write an `f64`, 1900 with 255
  digits after the point, and 3830 to read 801 digits, of 4096. Past 800
  significant digits, reading keeps one more, a 1 if any after is not 0,
  which is strictly between what's left and what's next, so rounds as the
  whole does.
- **F7 (the same everywhere).** The interpreter, const eval and C each
  round every operation once: C is compiled with `-ffp-contract=off`, so
  no `a * b + c` is fused, and never with fast-math; its literals are
  exact hex floats, or builtins of the infinities and NaNs with every bit.
  So a program prints the same digits on every machine, and a const has
  the value the same code has when it runs; but for a NaN's bits.

## What it doesn't do

- **Float refinements.** Floats aren't in the logic, so nothing is proved
  of one: not that it isn't a NaN, nor that it's in a range.
- **`%`, `mul_add`, math functions** (`exp`, `ln`, `sin`, `tanh`): the
  next step writes those in T, so they're the same on every machine, which
  the C library's aren't.
- **Fast floats.** No fast-math: "no NaN" is undefined behaviour in C,
  which could delete the range check of `x as usize`, and reordering sums
  would make the interpreter, const eval and C disagree. Sums that
  vectorise are for a library with an order of its own.
- **Floats in atomics and `Volatile`**, hex float literals, and a total
  order wrapper that's `Eq` and `Ord`.
- **Faster text.** Ryu and Eisel-Lemire would write and read in a few
  steps; the big numbers take hundreds, which is fine for now.

## Decisions

- **D1: operators only.** IEEE 754's comparisons as the language's
  operators, and floats neither `Eq` nor `Ord`, whose laws they'd break;
  rather than `Eq` without reflexivity, a total order as `==`, or a
  `PartialEq` and `PartialOrd`.
- **D2: `as` is the same number, or a panic**, as for the integers;
  rounding is named.
- **D3: literals as the integers'**, with no default type.
- **D4: no refinements of floats** at first.
- **D5 (Claude's): text in T**, on exact big numbers, rather than trusting
  the interpreter's Rust and a C library of the backend's, which would
  need one of its own for the freestanding target. Ties of the shortest
  digits go up, as Rust's do, so Rust is the tests' oracle.

## Exercises

1. Why does `0.1 + 0.2f32` print `0.3`, when `0.1 + 0.2f64` doesn't?
2. Write `fn lerp<F: Float>(a: F, b: F, t: F) -> F`. Is `a + (b - a) * t`
   exactly `b` at `t = 1.0`? Is `(1.0 - t) * a + t * b`?
3. `x.round() as i64` may panic. For which `x`? Write a `checked_round`
   that gives `None` instead, with no `with panic`.
4. Why can't the refinement logic know that `x < y` and `y < z` give
   `x < z` of floats, though IEEE 754's `<` is transitive?
5. `{x:.2}` of `2.675` is `2.67`. Find the `f64` nearest 2.675, and say
   why.

## Reading

- IEEE 754-2019, *Standard for Floating-Point Arithmetic*: the formats,
  rounding, `minimum` and `maximum`, `totalOrder`.
- David Goldberg, *What Every Computer Scientist Should Know About
  Floating-Point Arithmetic* (1991).
- Robert Burger and R. Kent Dybvig, *Printing Floating-Point Numbers
  Quickly and Accurately* (1996), and Guy Steele and Jon White, *How to
  Print Floating-Point Numbers Accurately* (1990).
- William Clinger, *How to Read Floating Point Numbers Accurately* (1990).
- Ulf Adams, *Ryū: Fast Float-to-String Conversion* (2018), and Daniel
  Lemire, *Number Parsing at a Gigabyte per Second* (2021): what's faster.
- Rust RFC 3514, *Float semantics*: which NaN an operation gives isn't
  said.
- John Gustafson, *Beating Floating Point at its Own Game: Posit
  Arithmetic* (2017): a format whose one NaR and one zero make `==` an
  equivalence, so a posit type could be `Eq` and `Ord`.
