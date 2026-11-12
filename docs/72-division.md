# Step 72: dividing by a constant

Code: `Walk::divide`, `Walk::logic_op`, `truncated`, `Checker::divisor`
and `constant` in `src/check/refine.rs`; `Kind::Integer`, `range_of` and
`Lin::text` in `src/check/refine/logic.rs`; `pred_op` in `src/interp.rs`.

t-metal prints a number in hexadecimal a digit at a time, four bits down
each round. Without an invariant, `shift - 4` may underflow, and the one
that says why couldn't be written:

```text
let mut shift: u32 = 64;
while shift > 0 invariant shift % 4 == 0 {
    shift = shift - 4;
    ...
}
```

```text
type error: an invariant can't use `%` yet: only `+`, `-` and `*` by a constant
```

A predicate's logic was linear arithmetic on the integers, and `%` wasn't
in it. Division by a constant is linear, though: `a / 4` is the integer
`q` with `4 * q` at most 3 from `a`, on the side of zero. That's
*Presburger arithmetic*, the integers with `+` and division by constants
(docs/08-refinement-types.md listed it as missing). Now a predicate, a
loop invariant and a measure's body can divide by a constant, and the loop
is proved:

```rust
fn digits() -> u64 {
    let mut shift: u32 = 64;
    let mut count: u64 = 0;
    while shift > 0 invariant shift % 4 == 0 && shift <= 64 && count == 16 - shift / 4 {
        shift = shift - 4;
        count = count + 1;
    }
    count
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&digits())?; // expect: 16
    Ok(())
}
```

At the head, `shift % 4 == 0` and `shift > 0` say `shift` is 4, 8, ...:
at least 4, so `shift - 4` fits a `u32`.

## What a predicate can divide by

A constant, which isn't 0: `v % 4`, `v / -2`, `v / (2 * 8)`. It's of
literals, worked out where the predicate is checked, so a predicate never
divides by zero, and stays total:

```rust,error
fn f(a: { v: i64 | v % (2 - 2) == 1 }) {}
// error: a refinement can't divide by zero
```

Not by an unknown: `v % n` isn't linear.

```rust,error
fn f(n: i64, a: { v: i64 | v % n == 1 }) {}
// error: a refinement can use `%` only by a constant: `n` isn't one
```

A const item isn't a constant of literals either, for now: its value is
the checker's only once its own body is checked (see "What it doesn't
do").

## What it means

What `/` and `%` mean in a body: `/` rounds towards zero, and `%` has the
sign of what's divided, so `a == (a / k) * k + a % k`. By a negative
constant too: `7 / -2` is -3, and `7 % -2` is 1. A predicate's integers
are the mathematical ones (docs/08-refinement-types.md), so its `/` never
overflows, even of the least `i64` by -1.

```rust
fn seven(a: { v: i64 | v == -7 }) -> {
    r: i64 | r / 2 == -3 && r % 2 == -1 && r / -2 == 3 && r % -2 == -1
} {
    a
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&seven(-7))?; // expect: -7
    Ok(())
}
```

Rounding down instead, as Python's `//` and `%` do, would make `-7 / 2`
-4 and `-7 % 2` 1; claiming either is refused
(`tests/t/err_soundness_pred_div_floor.t`,
`tests/t/err_soundness_pred_rem_sign.t`).

## One quotient

The logic has no `/`: it has sums, compared. So `a / k`, for a sum `a`, is
a variable `q` of its own, and a *law* says which integer it is. For `m =
|k|`:

- if `a >= 0`, then `m * q <= a <= m * q + m - 1`;
- if `a <= 0`, then `m * q - m + 1 <= a <= m * q`.

`a / k` is `q`, or `-q` if `k < 0`, and `a % k` is `a - m * q`. If the
ranges of `a`'s variables say its sign (it's a `u64`, say), the law is
only that half.

There's one `q` for each sum and `m`, wherever it's said: in a predicate
assumed, a predicate proved, or a body's `/` or `%`. That's what makes
assuming and proving meet. Before, a body's `%` by a constant was only
known to be less than the divisor, unrelated to what it divided, so this
couldn't be proved:

```rust
fn even(n: { v: u64 | v % 2 == 0 }) -> { r: u64 | r % 2 == 0 } { n + 2 }

fn next_even(n: u64) -> u64 {
    if n % 2 == 0 && n < 100 { even(n) } else { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&next_even(4))?; // expect: 6
    Ok(())
}
```

The branch's `n % 2 == 0` is `n - 2 * q == 0`; `even`'s precondition is
the same formula of the same `q`.

A quotient is one unknown, so `(x * y / 2) * z` is a product of two
(docs/51-products.md): a quotient of a product, times `z`.

## Measures

A measure's body is a term of the same logic, so it can divide by a
constant too:

```rust
#[derive(Copy, Discard)]
struct Board { width: { v: u64 | v % 2 == 0 && v <= 64 } }

impl Board {
    #[measure]
    fn half(&self) -> u64 { self.width / 2 }
}

fn left(b: Board, x: { v: u64 | v < b.half() }) -> { r: u64 | r < b.width } { x + b.half() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let b = Board { width: 10 };
    let mut out = os.stdout;
    out.line(&left(b, 4))?; // expect: 9
    Ok(())
}
```

## When a proof fails

A quotient is named by what it divides, in parentheses:

```text
type error: can't prove `v % 4 == 0` for argument `a` of `need`
  = note: required condition: x == 4 * (x / 4)
  = note: known here: 3 <= (x / 4); 4 * (x / 4) <= x && x <= 4 * (x / 4) + 3; ...
```

## Why it's sound

- **Q1.** For `m >= 1` and any integer `a`, exactly one integer `q`
  satisfies the law: the integers' quotient of `a` by `m`, rounded
  towards zero. If `a >= 0`, it's the greatest `q` with `m * q <= a`;
  if `a <= 0`, the least with `a <= m * q`; at `a = 0` both are 0.
- **Q2.** So the law, of a variable `q` nothing else mentions yet, is a
  *conservative extension*: every state the facts allow extends to one
  the law allows too, by taking `q` to be that quotient, so it proves
  nothing new of anything else; and in every state it allows, `q` is that
  quotient. It holds of values, not of places, so it holds on every path
  (`Walk::laws`), and two quotients of one sum by one `m` are equal, so one
  variable for both is sound.
- **Q3.** T's `/` and `%`, where they don't panic, are the integers' `/`
  rounding towards zero and the remainder with `a == (a / k) * k + a % k`
  (docs/11-integers.md); `a / -m == -(a / m)` and `a % -m == a % m`. So a
  body's quotient, a predicate's and the interpreter's (`i128`'s
  `checked_div`, which also rounds towards zero) agree, and what a
  predicate assumes is what a body proves.
- **Q4.** A predicate divides only by a constant that isn't 0, checked
  before any proof: so it's total, and in the logic.

A quotient's variable has no range of its own (`Kind::Integer`): the law
and `a`'s ranges bound it. Giving it a machine type's would be wrong in a
predicate, whose sums can be beyond every type's
(`tests/t/err_soundness_pred_div_range.t`). In a body, `x / -1` of the
least `x` panics, and on the path where it didn't, `-x` is what the logic
says it is: no fact is false there.

`tests/generated_division.rs` checks quotients and remainders of values
from the least `i64` to the greatest by divisors of either sign against
the host's arithmetic, that a claim one off is refused, and that what a
remainder and a range assume pins a value down exactly when one integer
is both.

## What it doesn't do

- **Divide by a const item or a const parameter.** `v % PAGE` waits for
  the const's value to be known where predicates are checked, which is a
  question of the order the checker does things in (docs/64-phase-order.md);
  `v % N` isn't linear.
- **`>>`, `<<` and `&` by constants.** `x >> 2` is `x` divided by 4
  rounding down, not towards zero, and `x & 3` is a remainder only of a
  non-negative `x`: each is a law away, but none was asked for yet.
- **Decide all of Presburger arithmetic.** The procedure is Fourier–Motzkin
  with each derived constraint rounded as integers allow
  (docs/08-refinement-types.md), which is sound and incomplete:
  `v % 3 == 1 && v % 2 == 0 && 0 < v && v < 4` has no integer, but has
  rational solutions, and the procedure doesn't see there's none. Pugh's
  Omega test (its "dark shadow" and "splinters") would.

## Decisions

- **D1: divisors are constants of literals.** Not const items, whose value
  is known only after their bodies are checked, nor const parameters,
  which aren't linear. Either can come later.
- **D2: measures can divide too.** A measure's body is a term of the same
  logic as a predicate.
- **D3: a negative constant divides as `/` does.** No rule bodies don't
  have.

## Exercises

1. Prove Q1 for `a <= 0`: why is the least `q` with `a <= m * q` the
   quotient rounded towards zero?
2. Make the law its first half alone, unguarded, whatever `a`'s sign.
   Which test in `tests/t` fails, and what does `q` then mean for a
   negative `a`? Why is that half alone right when `range_of` says `a >=
   0`?
3. Give every quotient its own variable, with no memo. Which tests still
   pass, and why are they proved anyway? (Hint: two variables with the
   same law are equal by Q1, and the procedure can show it.)
4. `v % 3 == 1 && v % 2 == 0 && 0 < v && v < 4` isn't refuted. Write out
   the constraints the procedure sees, and find a rational solution.
5. What would `x >> 2`'s law be, for `x` of either sign?

## Reading

- Presburger, *Über die Vollständigkeit eines gewissen Systems der
  Arithmetik ganzer Zahlen* (1929): the decidability of the integers with
  addition, the logic this step completes the operators of.
- Cooper, *Theorem Proving in Arithmetic without Multiplication* (1972):
  quantifier elimination for Presburger arithmetic.
- Pugh, *The Omega Test: a fast and practical integer programming
  algorithm for dependence analysis* (1991): Fourier–Motzkin made
  complete for the integers.
- Bradley and Manna, *The Calculus of Computation* (2007), chapter 7:
  Presburger arithmetic and its decision procedures.
