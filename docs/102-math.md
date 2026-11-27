# Step 102: math functions

Code: `core/float/math.t` (each function's fast path, and `decide64` and
`decide32`, which say whether it's enough), `core/float/fix.t` (`Fix`,
`decide` and `round`, the slow paths, `reduce` and `pow_exact`), the new
methods of `Float` (`core/num.t`), `src/mono/promote.rs` (consts read
where they are), and `scripts/math` (the checks against MPFR, and the
tables).

`Float` has the functions of numerical code: `exp`, `exp2`, `exp_m1`,
`ln`, `log2`, `log10`, `ln_1p`, `pow`, `sin`, `cos`, `tan`, `sinh`,
`cosh` and `tanh`, of `f32` and `f64`. Each gives the float nearest the
exact value, ties to even, for every input: it's *correctly rounded*, as
IEEE 754-2019 recommends (§9.2), and as the operators are.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1.0f64.exp())?;             // expect: 2.718281828459045
    out.line(&2.0f64.ln())?;              // expect: 0.6931471805599453
    out.line(&9.0f64.pow(0.5))?;          // expect: 3.0
    out.line(&(-2.0f64).pow(3.0))?;       // expect: -8.0
    out.line(&(-8.0f64).pow(1.0 / 3.0))?; // expect: NaN
    out.line(&1e22f64.sin())?;            // expect: -0.8522008497671888
    out.line(&0.5f32.tanh())?;            // expect: 0.46211717
    out.line(&1000.0f64.log10())?;        // expect: 3.0
    Ok(())
}
```

`(-8.0).pow(1.0 / 3.0)` is a NaN: `1.0 / 3.0` isn't a third, and no
negative number has a real power that isn't an integer. Each function is
`with {}`, and the checker proves none of them panics.

## Correctly rounded

A function correctly rounded has one answer, which mathematics fixes,
not code: so it's the same on every machine, and of every correct way of
working it out. The C library's functions aren't: they're within an ulp
or so, and which way they're off depends on the library, its version and
the machine, so a program's output does too. T's were written once, in T,
and could have been made the same everywhere by that alone, as the
interpreter and C run the same code; but a faster version later, with
FMA, vectors or on a GPU, would then change every program's output. A
correctly rounded one may be made faster freely: only the answer is
fixed.

It also keeps what's true of the exact functions that rounding can keep.
Rounding to nearest is monotone, so `exp` and `ln` are, and odd
functions stay odd: `(-x).sin()` is `-x.sin()`, bit for bit. `exp(0)` is 1,
`ln(1)` is 0, `log2` of a power of two is its exponent, and `pow` is exact
wherever `x^y` is a float.

`exp_m1` and `ln_1p` are `e^x - 1` and `ln(1 + x)`, rounded once: precise
where `x` is near 0, where `x.exp() - 1.0` rounds twice and loses what
matters:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1e-10f64.exp_m1())?;      // expect: 1.00000000005e-10
    out.line(&(1e-10f64.exp() - 1.0))?; // expect: 1.000000082740371e-10
    Ok(())
}
```

## The values at the edges

What IEEE 754-2019 says (§9.2.1), which C's Annex F says too:

- `exp` is 1 of 0, infinity past about 709.78 (88.72 of an `f32`), and 0
  below about -745.13 (-103.97): of what's between, the subnormals too,
  the nearest. `exp2` of an integer is exactly that power of two.
- `ln`, `log2` and `log10` are minus infinity of `±0.0`, a NaN below 0,
  and 0 of 1; `ln_1p` is minus infinity of -1 and a NaN below it.
- `pow(x, y)` is 1 of `y = ±0` and of `x = 1`, even where the other is a
  NaN; else a NaN if either is. Of a negative `x`, it's a NaN unless `y`
  is an integer, and then `|x|^y`, negative where `y` is odd. Of `±0.0`,
  it's an infinity where `y` is negative, and 0 where it's positive, with
  `x`'s sign where `y` is an odd integer; of `±∞` the other way round; of
  `y = ±∞`, 1 of `x = -1`, and 0 or infinity as `|x|` is below or above
  1.
- `sin`, `cos` and `tan` of an infinity are a NaN, of any finite `x`, in
  radians, however big, as exactly as of a small one: `1e22.sin()` above is
  the sine of 10^22 itself, which is an `f64`.
- `tanh` is ±1 past about 19.1 (9.1 of an `f32`).

A NaN in gives a NaN out, but where `pow` says otherwise; which NaN,
its sign and the rest of its bits, isn't said, as for the operators.

## How they're worked out

Each function is worked out first on a *fast path*, in double-double: a
value as the sum `hi + lo` of two `f64`s, about 106 bits, with no FMA, so
the products are Dekker's, exact. What it gives is within a bound the
function states of the exact value: 2^-72 of it for `exp`, 2^-78 for `ln`,
2^-67 for `pow`, and so on. If every value that close rounds to the same
float, that's the answer; and nearly always it does, as the bound is more
than 10 bits past the float's last.

Where it doesn't, the exact value is within the bound of halfway between
two floats, and the *slow path* works it out again in fixed point, to 288
bits, and then, if that isn't enough either, to 1056: integers of 32-bit
limbs, by series (`e^r` by Taylor's, `ln m` as `2 atanh z`, the sine and
cosine by theirs), with an error bound added up step by step. It runs
about once in 2^14 calls, or more rarely, so it costs nothing on average.

An `f32`'s function is its `f64`'s fast path, rounded to an `f32`: far
more precise than it needs, so it decides every input but a handful, and
an `f32` path of its own can be made faster later, giving the same
answers.

`sin`, `cos` and `tan` reduce `x` by the nearest multiple of π/2: below
2^20 by π/2 in four parts (Cody and Waite's way), and above, by the bits
of 2/π that matter, up to 2091 of them (Payne and Hanek's way), so the
sine of 10^22 is as exact as of 1.

## Tables

The fast paths use tables, of `2^(j/64)` for `exp`, of `1/c` and `ln c` for
`ln`, of `sin` and `cos` of `j/64`: consts, worked out by MPFR
(`scripts/math/tables.c`). A use of a const is its value (docs/36), which
a body built at each use, the whole table for one entry. Now a body that
only reads an aggregate const, `TABLE[i]` or `CONST.field`, reads the one
copy there is of it, in the program's data, as Rust promotes `&CONST`:
`exp` went from 61 ns to 18 that way. It's the same for every program's
consts.

## Checking stays quick

The functions are about 2,700 lines of core, which every `t check`
reads, though most programs call none of them. Since 0.94.2 a check of a
program types the libraries' bodies and checks in full only what it can
see of them (`Checker::only_typed`); now it does less still:

- A library function with no type parameters isn't typed at all, but
  for its signature: nothing in its body reaches a program's check
  (lemma Q1).
- A library const that isn't exported is only checked when the
  libraries are checked in full, as no program can name it (Q2).
- Of the consts' values, a check works out only the program's, and what
  they need: a library's matter only to running, and never fail (Q3).

Checking and running a program in full, as `t run`, `t build` and the
tests do, does less with them too: a library's functions without type
parameters are copied for a program where it calls them, as generic ones
always were, rather than all of them for every program and the unused
ones dropped again (`Mono::run`); and the libraries' consts' values,
the same in every program, are worked out once in a process, not once a
program (`LIBRARY_CONSTS`).

`t check` of an empty program went from 125 ms to 133 on this machine,
for the 2,700 lines, where it would have been about 210, and of t-chess
from 468 to 490; `t run`, which checks the libraries in full first, from
0.58 s to 0.83, until each library's check is kept on disk
(`BACKLOG.md`).

## How it's checked

Against MPFR (`scripts/math/check.sh`), which rounds the exact value once
at the format's precision:

- every one of the 2^32 inputs of each function of an `f32` but `pow`;
- of each of an `f64`, millions of random inputs, as bits, in ranges
  where the function does something, and near its edges;
- of `pow`, of both types, millions of random pairs and 1.7 million
  chosen ones: exact powers, values halfway between two floats, the
  powers of two, negative `x`, and every special value against every
  other.

The tests carry inputs MPFR found hardest of each function, among 300
million drawn at random, which the slow path decides, each with MPFR's
answer, and run them in the interpreter and natively
(`tests/generated/math.rs`). They catch a table entry or a term of a
series wrong, and `pow_exact` left out; but not a fast path's rounding
test turned off, as even the hardest of random inputs are rounded right
by a fast path that accurate. Of an `f32`, the check of every input
does; of an `f64`, only the hardest inputs known would, which searches
built for it find. So a change to the functions is checked against MPFR
before it merges (`CLAUDE.md`).

## Speed

A few times slower than the C library's, which use FMA: `exp` 18 ns to
glibc's 5, `ln` 27 to 3, on this machine. An `f32`'s is about as fast as an
`f64`'s, as it's the same path. Making them faster is the next step's,
with the CPU's speed measured first (`BACKLOG.md`).

## Why it's right

- **M1 (fast paths).** Each fast path's error is below its stated bound
  with a margin of at least 8: the terms of each series it leaves out,
  bounded by the next term, and each operation's rounding, at most half
  an ulp of its result, added up where it's worked out (each function's
  doc comment says how). Checked by MPFR: `exp`'s greatest error over 8
  million inputs was 2^-75.7, of a bound 2^-72; `ln`'s 2^-82.5, of 2^-78.
- **M2 (the rounding test).** Rounding to nearest is monotone: if the
  exact value is within `[v - e, v + e]` and both ends round to the same
  float, so does it. `decide64` works out the ends as `hi + (lo ± e)`,
  each rounding of which moves them by far less than the margin, so
  they're still on the far side of the exact value; `decide32` rounds
  `hi + lo` and the ends to an `f64` first, and widens the interval by
  the two roundings that may move them, 2^-53 of the value each. Only a
  normal result is decided this way: a subnormal rounds at fewer bits,
  and goes the slow way.
- **M3 (slow paths).** Each operation on a `Fix` is exact or truncated,
  so less than a unit, `2^-F`, off; each series is summed until its
  terms are 0, and what's left out then is below a unit; and each slow
  path adds up what its parts may be off by into the bound it decides
  with, which `decide` uses as M2 says, exactly, on integers. 288 bits
  decide every input whose exact value isn't within about 2^-250 of
  halfway between two floats, and 1056 bits every one not within 2^-1000.
  The searches for the hardest inputs of these functions (Lefèvre and
  Muller's, and after them Zimmermann's) have found none closer than
  about 2^-120; an input past both would round as the 1056 bits do
  (`fix::nearest`), which no input known does.
- **M4 (`pow`'s exact values).** `x^y` is a float or halfway between two
  only where `pow_exact` works it out exactly, before the paths that
  can't tell. With `x = mx · 2^ex`, `mx` odd, and `y = n / 2^g`, `n` odd: if
  `mx = 1`, `x^y = 2^(ex·y)`, which is a power of two if `ex · y` is an
  integer, and irrational otherwise. Else `mx^y` is dyadic only if `y > 0`,
  and rational only if `mx` is some `r^(2^g)`; then `x^y = r^n ·
  2^(ex·y)`, which is a float or halfway between two only if `r^n`, odd,
  has at most `p + 1` bits, so `n ≤ 34` and `2^g ≤ 32`, as `r ≥ 3`. So a
  `y` from 1/32 to 64, a multiple of 1/32, and a perfect power `mx`: those
  `pow_exact` tries, with integers; all others are neither, so their
  rounding is decided by the paths that assume so.
- **M5 (no panic).** Every function is `with {}`, checked: the integer
  arithmetic of the slow paths is proved in range, and float operations
  can't panic (docs/101, F2). A slow path's loops stop at a bound, which
  M3's sizes are within.
- **M6 (the same everywhere).** Only IEEE 754's operations and integers,
  each rounded once (docs/101, F7), so the interpreter, const eval and C
  give the same bits; and as each answer is the correctly rounded one,
  so does any other correct implementation.
- **P1 (consts read in place).** A const's value is plain data, which
  has no `Cell` (docs/36, C4): so a body that never writes the temporary
  holding a use's value, nor borrows it `&mut`, nor uses it up, can't
  tell the copy it reads from the static's, which is the same value for
  the whole run, as the interpreter checks every write. `mono::promote`
  turns only such temporaries, of aggregates, into the `&'static` a
  static's use is.

- **Q1 (functions not typed).** What typing a library body gives a
  program's check is the calls it makes, at which type arguments, which
  polymorphic recursion is about, and its closures' types
  (`Checker::only_typed`). A
  function with no type parameters makes each call at type arguments its
  body fixes, so no cycle of instances growing a type goes through it;
  its closures are its own; and it says its effects
  (`library_functions_say_their_effects`), so nothing is inferred of it.
  A const or a run needs its body: then the libraries are checked in
  full first, as before. `check_typed_says_what_check_says` checks both
  ways over every test program.
- **Q2 (private consts).** What the logic knows of a const is for code
  that names it; a library's that isn't exported only its own library
  names, whose bodies a program's check doesn't verify. Statics, which
  a program may reach through what the library exports, are checked as
  before.
- **Q3 (consts' values).** A const's value is worked out once its
  program checks, for running it; a check alone needs only the
  program's, whose evaluation may fail, and those finds what it needs of
  the libraries' as it goes (`crate::consts`). A library's const can't
  fail: every run works it out.

Writing the slow paths found one hole in what the checker knew, not in
what it proved: a `let mut` with a refinement, changed in a loop that
`break`s, lost its refinement after the loop, as the loop's end, which
never read the variable, said nothing of it where the paths met. Every
write proves the refinement, so it's now assumed of the joined value too
(lemma J3 of docs/61-proof-search.md), but not of a `let x;` declared
with no value.

## What it doesn't do

- **The rest of the functions**: `asin`, `acos`, `atan`, `atan2`,
  `asinh`, `acosh`, `atanh`, `cbrt` and `hypot`, the next step, on the
  same machinery; then `sin_pi` and `cos_pi`, which IEEE 754 recommends
  too, and Box–Muller wants.
- **Speed**: no FMA, as T has no `mul_add` yet, so each exact product is
  Dekker's 17 operations, not two; no `f32` paths of their own; and above
  2^20 the trigonometric functions reduce in fixed point, about a
  microsecond.
- **What the logic knows of them**: floats aren't in the refinement logic
  (docs/101, D4), so that `exp` is monotone, which correct rounding
  makes true, isn't anything a proof can use yet.

## Decisions

- **D1: correctly rounded**, every function, both types: the answer is
  mathematics', so every implementation agrees and any may be made
  faster; rather than within an ulp, which the code alone would fix.
- **D2: IEEE 754's `pow`**, which is C's and Rust's `powf`: `pow(NaN, 0)`
  and `pow(1, NaN)` are 1; rather than `powr`'s NaN in, NaN out.
- **D3 (Claude's): written in T**, over IEEE 754's operations and
  integers, double-double then fixed point; no C library, which the
  freestanding target hasn't, and no FMA, which not every machine has.
- **D4 (Claude's): Rust's names**: `ln`, `ln_1p`, `exp_m1`, and `pow`, not
  `powf`, as `Float` has only the one kind of power.
- **D5 (Claude's): fourteen now**, the ones the ML path needs, and the
  other nine next, rather than all of them in one step.

## Exercises

1. `exp(x)` is worked out as `2^m · 2^(j/64) · e^r`. Why does `r`, at most
   about 0.0054, need `ln 2 / 64` in two parts, one of 36 bits?
2. Why can't `sin(x)` be 0 for any float `x` but 0? What's the least
   `|sin(x)|` of an `f64` `x` of magnitude at least 1, roughly?
3. `pow(x, 2.0)` can be exactly halfway between two `f64`s. Find an `x`
   for which it is, and say which way it rounds.
4. `decide32` widens the interval by `2^-52 · |y|`. Show that `2^-53`
   wouldn't be enough.
5. Why is `tanh` of an `f64` 1 from 19.1 on, but `sinh` not infinity until
   about 710.48?

## Reading

- IEEE 754-2019, §9.2: the recommended operations, correctly rounded,
  and their special values.
- Jean-Michel Muller, *Elementary Functions: Algorithms and
  Implementation*, 3rd ed. (2016): range reduction, tables, the
  Table-Maker's Dilemma.
- Abraham Ziv, *Fast Evaluation of Elementary Mathematical Functions
  with Correctly Rounded Last Bit* (1991): the fast path and its test.
- Vincent Lefèvre and Jean-Michel Muller, *Worst Cases for Correct
  Rounding of the Elementary Functions in Double Precision* (2001).
- Alexei Sibidanov, Paul Zimmermann and Stéphane Glondu, *The CORE-MATH
  Project* (2022): correctly rounded functions in C, as fast as the
  usual ones.
- Mary Payne and Robert Hanek, *Radian Reduction for Trigonometric
  Functions* (1983); W. J. Cody and W. Waite, *Software Manual for the
  Elementary Functions* (1980).
- T. J. Dekker, *A Floating-Point Technique for Extending the Available
  Precision* (1971): double-double, and the exact product.
- Christoph Lauter and Vincent Lefèvre, *An Efficient Rounding Boundary
  Test for pow(x, y) in Double Precision* (2009): when `x^y` is exact.
