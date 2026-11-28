# Step 103: more math functions

Code: the end of `core/float/math.t` (each function's fast path, and
the comparisons that decide `cbrt` and `hypot`), the end of
`core/float/fix.t` (the slow paths, `sqrt_fix`, `atan_fix` and `ln_of`,
and `cube_above` and `square_side`), the new methods of `Float`
(`core/num.t`), and `scripts/math` (the checks against MPFR, which now
know these functions too).

`Float` has the rest of the functions of numerical code: `asin`, `acos`,
`atan`, `atan2`, `asinh`, `acosh`, `atanh`, `cbrt` and `hypot`, and
`sin_pi` and `cos_pi`, of `f32` and `f64`. Like step 102's, each gives the
float nearest the exact value, ties to even, for every input (D1 of
docs/102-math.md): the same bits on every machine, and of any correct
implementation. Each is `with {}`, and the checker proves none of them
panics.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1.0f64.atan2(-1.0))?;    // expect: 2.356194490192345
    out.line(&0.5f64.asin())?;         // expect: 0.5235987755982989
    out.line(&(-27.0f64).cbrt())?;     // expect: -3.0
    out.line(&3.0f64.hypot(4.0))?;     // expect: 5.0
    out.line(&1e300f64.hypot(1e300))?; // expect: 1.4142135623730952e300
    out.line(&0.5f64.atanh())?;        // expect: 0.5493061443340549
    out.line(&0.25f64.cos_pi())?;      // expect: 0.7071067811865476
    let x: f64 = 1e15 + 0.5;
    out.line(&x.sin_pi())?;                    // expect: 1.0
    out.line(&(x * 3.141592653589793).sin())?; // expect: 0.9525593659368664
    Ok(())
}
```

`cbrt(-27.0)` is -3 exactly, as every exact cube root is; the C
library's is within an ulp, and glibc's is `-3.0000000000000004`.
`hypot` never overflows on the way: `1e300 * 1e300` would.

## `sin_pi` and `cos_pi`

`x.sin_pi()` is `sin(πx)`, worked out from `x` itself. `(x * π).sin()`
isn't the same: `π` as a float isn't π, and the product is rounded, so
of an `x` of 10^15 the angle is off by a third of a radian, as above,
and of bigger ones by more; `sin_pi` reduces `x` by 2 exactly, as a float minus the nearest
integer or half is exact, and is as precise of 10^15 + 1/2 as of 1/2,
and exact there. IEEE 754-2019 recommends them (§9.2, `sinPi` and
`cosPi`), and they're what a program that works in turns wants: the
cosine half of Box–Muller is `cos(2πu)`, `(2.0 * u).cos_pi()`, and the
twiddle factors of an FFT are `cos_pi(2k/n)`.

## The values at the edges

What IEEE 754-2019 says (§9.2.1), which C's Annex F says too:

- `asin` and `acos` are a NaN past ±1, `asin(±1)` is ±π/2, `acos(1)` 0 and
  `acos(-1)` π. `atan(±∞)` is ±π/2. Each of these keeps the sign of a
  zero: `asin(-0.0)` is `-0.0`.
- `y.atan2(x)` is the angle of the point `(x, y)`, from -π to π, its sign
  always `y`'s, `-0.0`'s too. Where `y` is ±0, it's ±0 if `x` is positive
  or `+0.0`, and ±π if it's negative or `-0.0`; where `x` is ±0 and `y`
  isn't, ±π/2; of an `x` of ±∞ and a finite `y`, ±0 or ±π; of a `y` of ±∞,
  ±π/2, or ±π/4 and ±3π/4 where `x` is infinite too.
- `acosh` is a NaN below 1 and 0 of 1; `atanh(±1)` is ±∞, and a NaN past
  them.
- `cbrt` of ±0 and ±∞ is itself.
- `x.hypot(y)` is +∞ if either is infinite, even where the other is a
  NaN, as the answer is ∞ whatever the NaN stands for; else a NaN if
  either is. Of two zeros it's `+0.0`, whatever their signs.
- `sin_pi` of an integer is 0, with the integer's sign, `(-2.0).sin_pi()`
  `-0.0`, so that `sin_pi(-x)` is always `-sin_pi(x)`; `cos_pi` is +0
  halfway between two integers, `cos_pi(n + 1/2)` for every `n`.

## How they're worked out

On step 102's machinery: a fast path in double-double, about 106 bits,
within a bound each function states of the exact value, and `decide64`
or `decide32` to say whether that bound is enough to know the nearest
float; where it isn't, about once in 2^22 calls, a slow path in fixed
point, to 288 bits and then 1056.

- **`atan`** has a table of `atan(j/64)`, `j` from 0 to 64. Of a `t` from
  0 to 1, `c = j/64` is the nearest, and `atan t = atan c + atan u`, `u =
  (t - c)/(1 + tc)`, at most 1/128, whose series is short. Above 1, it's
  `π/2 - atan(1/t)`. Nothing cancels: `atan u` is at most half of `atan
  c`, and `π/2 - atan(1/t)` is at least π/4.
- **`asin` and `acos`** are `atan`s: `asin a = atan(a / s)`, `s = sqrt(1 -
  a^2)`, or `π/2 - atan(s / a)` past 1/√2, so the quotient is never above
  1, and `acos` the same parts, the other way round. Near 1, `1 - a^2` is
  worked out as `(1 - a)(1 + a)`, as `1 - a` is exact there, so `acos` of a
  number just below 1, about `sqrt(2(1 - a))`, is as precise as any.
- **`atan2`** scales `x` and `y` by the same power of two, which is exact,
  so their quotient neither overflows nor underflows; then `atan` of it,
  or of its inverse, and the quadrant. Where `|y/x|` is below 2^-60 and
  `x` positive, the angle is the quotient, but for less than 2^-118 of
  it: but not quite, as IEEE 754's division alone would round a quotient
  exactly halfway between two floats to even, where the angle, a little
  less, rounds down. The slow path knows the quotient is exact by
  integers, and rounds it less half a unit (lemma A1).
- **`asinh`, `acosh` and `atanh`** are made of `ln`'s fast path (step
  102's `ln_core`), with what would cancel taken out: `atanh x = (ln(1 +
  x) - ln(1 - x))/2`, of two logarithms of opposite signs; `asinh x =
  ln(1 + u)`, `u = x + x^2/(1 + sqrt(1 + x^2))`, which is `x + sqrt(1 +
  x^2) - 1` without its subtraction; `acosh` near 1 of `t = x - 1`, which
  is exact there, as `ln(1 + t + sqrt(t(2 + t)))`; and past 2^28, `ln(2x)
  ± 1/4x^2`. Their slow paths are logarithms of fixed-point numbers
  (`ln_of`), of the same parts.
- **`cbrt` and `hypot`** have no slow path of series. They're algebraic:
  where the fast path can't tell which of two floats is nearer, the
  value is near their midpoint `m`, and `m^3` against `x`, or `m^2`
  against `x^2 + y^2`, says which side of it the value is on, exactly, in
  fixed point, as `m` has only 54 bits (lemma R1). A cube root is never
  halfway between two floats (R2); a `hypot` can be, as `5` is of `3` and
  `4`: of `a = m^2 - n^2` and `b = 2mn`, `sqrt(a^2 + b^2)` is the odd
  integer `m^2 + n^2`, which past 2^53 is halfway between two `f64`s,
  and rounds to the even one. `hypot` of two subnormals is exactly the
  integer nearest `sqrt(A^2 + B^2)` times 2^-1074, `A` and `B` their
  multiples of it, which settles its rounding too.
- **`sin_pi` and `cos_pi`** reduce `x` exactly, as `n + k/2 + r`, `n` an
  integer and `|r| ≤ 1/4` (lemma S1), then are step 102's `sin` and `cos`
  of `πr`, which no longer needs reducing; of an `r` of 0 they're 0 or ±1,
  with IEEE's signs. Below 2^-52, `sin_pi x` is `πx` but for 2^-102 of
  it, which can be subnormal; the slow path works out `πx` scaled, to
  round it there.

An `f32`'s function is its `f64`'s fast path, rounded to an `f32`, as in
step 102, but for `hypot`, whose squares and their sum are exact as
double-doubles, and whose comparison is then exact in `f64`s alone.

## How it's checked

Against MPFR (`scripts/math/check.sh`), as step 102's:

- every one of the 2^32 inputs of each function of an `f32` but `atan2`
  and `hypot`;
- of each of an `f64`, millions of random inputs, as bits and in ranges
  where the function does something and near its edges, and of `atan2`
  and `hypot`, of both types, millions of random pairs, quotients below
  2^-1000 and subnormals among them;
- `hypot` of Pythagorean triples whose hypotenuse is halfway between two
  floats, which no random pair finds.

A fast path is so precise that random inputs, even the hardest of
millions, hardly ever reach the slow paths, nor the exact comparisons:
so each was checked again on its own, in a `t` built with the fast
paths' bound made 1, which sends every input the slow way, and with
`cbrt`'s and `hypot`'s made half an ulp, which sends half of them to
the comparison: about 2 million inputs of each, every one right.
That found the one bug, in step 102's `fix::quotient`, which gave half of
`a / b` where the two have the same exponent: no caller in 0.110.0 passed
two such, and `atan`'s and `asinh`'s slow paths do.

The tests carry the hardest inputs of each function MPFR found among 300
million drawn at random, with its answers, now with the infinities and a
NaN among the edges of every function (`tests/generated/math.rs`).

## Speed

## Checking stays quick

## Why it's right

Step 102's lemmas hold of these functions as they're written there: M2
(the rounding test), M3 (the slow paths add up their errors), M5 (no
panic) and M6 (the same bits everywhere). Of their own:

- **M1 (fast paths).** Each fast path's error is below the bound it
  decides with, `ARC_EPS`, 2^-76, by far more than step 102's margin of 8:
  so that the hardest of random inputs, which the tests carry, are
  decided by the slow paths and the comparisons, and test them, as step
  102's do theirs. Each fast path's own: `atan_core`'s 2^-93, the series
  it leaves out, 2^-101, and its tail's roundings in `f64`, a few of their
  last bits, of a term 2^-31 of `Q`'s and `Q`'s 2^-15.6 of `u`'s;
  `asin_pos`'s and `atan2`'s 2^-92, the quotient's 2^-100 and the square
  root's 2^-103 more, as `atan`'s relative error is at most its
  argument's (`d atan t / atan t ≤ dt / t`); `cbrt_core`'s 2^-100, of a
  last Newton step whose cube is within 2^-102; `hypot_core`'s 2^-103.
  The inverse hyperbolic ones are `ln_core`'s, and decide with its
  `LN_EPS`, 2^-78: what they add is below 2^-99, of sums whose terms are
  all of one sign, and `ln(1 + u)` is no more sensitive to `u` than `u`
  is; `sin_pi` and `cos_pi` are `sincos_core`'s, and decide with
  `TRIG_EPS`, 2^-74. Measured against MPFR, the greatest error of 400,000
  inputs in each range: `atan`'s 2^-95.4, `asin`'s 2^-95.5, `acos`'s
  2^-96.6, `cbrt`'s 2^-103.6, `asinh`'s 2^-83.0, `acosh`'s 2^-88.4,
  `atanh`'s 2^-81.8 and `sin_pi`'s 2^-82.3.
- **R1 (`cbrt` and `hypot` by comparison).** Where `decide64` fails, the
  ends it rounded, `l` and `h`, are floats next to each other, as its
  interval is far narrower than an ulp, and the exact value is between
  them; rounding to nearest picks `l` below their midpoint `m` and `h`
  above. `m` has 54 bits; of `cbrt`'s `u` from 1 to 8, `m` from 1 to 2,
  `m^3` has 162 bits after the point, and of `hypot`'s `x^2 + y^2`, `y`
  at least 2^-28 of `x`, at most 164: a `Fix` of 288 holds both exactly,
  so the comparison is exact.
- **R2 (no cube root halfway).** A value halfway between two floats is
  `m · 2^e`, `m` odd of `p + 1` bits; its cube is `m^3 · 2^3e`, `m^3` odd of
  more than `p` bits, so not a float: no `x` has it as its cube root. A
  square can be a sum of two: so `hypot`'s comparison has a third answer,
  equal, which rounds to the even one.
- **A1 (`atan2` of a tiny quotient).** Of `q = Q · 2^k`, `Q = ma / 2mb`,
  `atan q = 2^k (Q - B)`, `B > 0` at most 2^-118 of `Q`. `Q` is exact
  where `mb`'s odd part divides `ma`, with at most 54 bits; then if `B` is
  below a unit of the fixed point, `atan q` is strictly between `Q` less a
  unit and `Q`, both multiples of the unit, as every rounding boundary is:
  so it rounds as `Q` less half a unit, a point strictly between them.
  Else `B` is worked out, and the interval it gives is that of any other
  function.
- **S1 (`sin_pi`'s reduction).** Of an `a` below 2^52, `n = trunc(a)` and
  `f = a - n` are exact (a float less its integer part is), and so is `f -
  k/2`, `k/2` the nearest of 0, 1/2 and 1 to `f`, as it's within a factor
  of two of `f`, or 0 (Sterbenz); `n`'s parity is the last bit of `n +
  2^52`, exact. From 2^52 on, every float is an integer, and from 2^53 an
  even one.

## What it doesn't do

- **The other functions in turns**: `tan_pi`, `asin_pi`, `acos_pi`,
  `atan_pi` and `atan2_pi`, which IEEE 754 recommends too; no program
  has asked for one (`BACKLOG.md`).
- **Speed**: as step 102's, no FMA, and an `f32`'s function is an `f64`'s.
- **What the logic knows of them**: floats aren't in the refinement
  logic (docs/101, D4).

## Decisions

- **D1 (Claude's): Rust's names and order**, as step 102's D4:
  `y.atan2(x)`, the receiver the `y`, as C's `atan2(y, x)` and Rust's
  `y.atan2(x)`; `x.hypot(y)`; and IEEE 754's `sinPi` and `cosPi` as
  `sin_pi` and `cos_pi`.
- **D2 (Claude's): IEEE 754's values at the edges**, as step 102's D2:
  `hypot(∞, NaN)` is ∞, `atan2(±0, -0)` is ±π.
- **D3 (Claude's): `sin_pi` and `cos_pi` only** of IEEE 754's functions
  in turns: the ones programs use, Box–Muller and FFTs; the others on the
  backlog until one's wanted.
- **D4 (Claude's): `cbrt` and `hypot` decided by comparison**, not by a
  series to more bits: exact, shorter, and no slower where it runs.

## Exercises

1. `atan_core` takes `c = j/64` nearest `t`, so `|t - c| ≤ 1/128`. Why is
   `u = (t - c)/(1 + tc)` no bigger, and why does `atan c + atan u`
   cancel at most half of `atan c`?
2. `acos` of `1 - 2^-53`, the float below 1, is about `2^-26`. Why would
   `atan(sqrt(1 - a * a) / a)` be wrong in most of its bits, and how does
   `(1 - a)(1 + a)` help?
3. Find floats `y` and `x` whose quotient is exactly halfway between two
   subnormals, and say which way `y.atan2(x)` rounds, and `y / x`.
4. Show that `cbrt` is exact wherever its value is a float: what must
   `x` be?
5. Of `a = m^2 - n^2`, `b = 2mn`, find `m` and `n` with `a` and `b` below
   2^24 and `m^2 + n^2` odd and above 2^24: `hypot` of `a` and `b` as `f32`s
   is then halfway between two.
6. Why is `(2.0 * u).cos_pi()` as precise as `cos_pi` of anything, but
   `(2.0 * PI * u).cos()` not, of a `u` near 1/4?

## Reading

- IEEE 754-2019, §9.2: `sinPi`, `cosPi`, `atan2`, `hypot` and the rest,
  and their special values.
- Jean-Michel Muller, *Elementary Functions: Algorithms and
  Implementation*, 3rd ed. (2016): tables and argument reduction for the
  inverse functions.
- Brian Gladman, Vincenzo Innocente, John Mather and Paul Zimmermann,
  *Accuracy of Mathematical Functions in Single, Double, Double Extended,
  and Quadruple Precision* (updated each year): how far each C library's
  functions are from correctly rounded.
- The CORE-MATH project (docs/102-math.md's reading): its `cbrt`,
  `atan2` and `hypot`, correctly rounded, in C.
