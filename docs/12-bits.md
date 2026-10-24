# Step 12: bits

Code: the typing rules in `src/check/expr.rs` (T-Bits, T-Shift, T-Compl),
what the refinement logic knows of them in `src/check/refine.rs` (`bits`,
`bitwise`), the precedence in `src/ast.rs` (`BinOp::nests_in`) and
`src/parser.rs`, `Signed` and `Unsigned` in `src/traits.rs`, the wrapping
functions in `core/num.t`, and `src/interp.rs` (`Interp::arith`, `wrap`).

```rust
fn utf8_lead(c: { v: u32 | v <= 0x10FFFF }) -> u8 {
    if c < 0x80 {
        c as u8
    } else if c < 0x800 {
        (c >> 6) as u8 | 0xC0
    } else if c < 0x10000 {
        (c >> 12) as u8 | 0xE0
    } else {
        (c >> 18) as u8 | 0xF0
    }
}

fn utf8_cont(c: u32, shift: u32) -> u8 { ((c >> shift) & 0x3F) as u8 | 0x80 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&utf8_lead(0x20AC))?;    // expect: 226
    out.line(&utf8_cont(0x20AC, 6))?; // expect: 130
    out.line(&utf8_cont(0x20AC, 0))?; // expect: 172
    Result::Ok(())
}
```

That's the euro sign, `€`, as UTF-8's three bytes: masks, shifts and `as`,
none of which can panic here, and the checker proves it. Neither function
says `with panic`.

## Five operators, and `!`

```text
  Γ ⊢ l : τ   Γ ⊢ r : τ   τ: Int         Γ ⊢ l : τ   Γ ⊢ n : u32   τ: Int
  -------------------------------- (T-Bits)  ---------------------------------- (T-Shift)
         Γ ⊢ l & r : τ                            Γ ⊢ l << n : τ

  for & | ^                                   for << >>

     Γ ⊢ e : τ   τ: Int
  ------------------------ (T-Compl)
       Γ ⊢ !e : τ
```

`&`, `|` and `^` are *and*, *or* and *exclusive or*, bit by bit. Both sides
have the same type, and so does the result, as for `+`. Only integers have
bits: on `bool`, `&&` and `||` say it, and `!=` is *exclusive or*.

Bits are of two's complement, the way every machine writes integers: a
number's bits are its binary digits, and a negative number has infinitely
many 1s on its left, so -1 is all 1s, and -2 is all 1s but the last. That's
a meaning in ℤ, not in a width, so step 11's rule still holds: an operator
gives the same number it gives in ℤ, or panics. And these never panic: the
result always fits (B1, below).

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: u8 = 0b1100;
    let b: u8 = 0b1010;
    out.line(&(a & b))?; // expect: 8
    out.line(&(a | b))?; // expect: 14
    out.line(&(a ^ b))?; // expect: 6
    let m: i8 = -1;
    out.line(&(m & 5))?; // expect: 5
    out.line(&(m ^ 5))?; // expect: -6
    out.line(&!0u8)?;    // expect: 255
    out.line(&!0i8)?;    // expect: -1
    Result::Ok(())
}
```

`!` of an integer flips all its bits. That's not ℤ's: of a `u8`, only its
eight bits flip, giving 255 for 0, where -1 would be all of them. It's the
type's least plus its greatest, minus the number: `255 - x` of a `u8`, and
`-1 - x` of an `i8`. That's in the type, and linear, so the logic knows
exactly what `!x` is.

`!` of a `bool` is as it was. Which `!` it is depends on its operand's type,
and only on that: if the operand's type isn't known yet, the checker waits
until it is, and if nothing ever says, it's "cannot infer", as for a
literal. It doesn't guess `bool`.

## Shifts mean what they mean in ℤ

`x << n` is `x` times 2^n, and `x >> n` is `x` divided by 2^n, rounded down:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(1u8 << 7))?;    // expect: 128
    out.line(&(-5i32 >> 1))?;  // expect: -3
    out.line(&(-5i32 >> 99))?; // expect: -1
    out.line(&(5i32 >> 99))?;  // expect: 0
    Result::Ok(())
}
```

That's the same for signed and unsigned types: there's no *arithmetic* and
*logical* shift to tell apart, as there is in C and Java, as rounding down
is what both do in two's complement. And `>>` never panics, however far it
shifts. In Rust, shifting an `i32` by 99 panics in a debug build and
shifts by 3 in a release one, as the machine only looks at the amount's
last five bits.

`<<` is multiplying, so it's under the rule for `*`: it panics if the
result doesn't fit, unless that's proved not to happen. It never drops bits
silently, as `as` never truncates. To drop them, mask first:

```rust,error
fn high_nibble(x: u8) -> u8 { x << 4 }
// error: `<<` may panic, but `high_nibble` doesn't say `with panic`
```

```rust
fn high_nibble(x: u8) -> u8 { (x & 0x0F) << 4 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&high_nibble(0xAB))?; // expect: 176
    Result::Ok(())
}
```

How far is a `u32`, whatever is shifted: it's a count, not a number of
`x`'s type, and it can't be negative. As literals have no default type, a
shift by any integer type would make `x << 3` "cannot infer"; a fixed type
makes `3` a `u32`.

## Parentheses, not precedence

```text
fn f(a: u8, b: u8, c: u8) -> u8 { a & b | c }
error: use parentheses to say which of `&` and `|` comes first
```

Every language has to say which of `a & b | c` and `a + b << c` comes
first, and they don't agree. In C, `x & mask == 0` is `x & (mask == 0)`, a
famous bug. Rust fixed that one, but `a + b << 2` is still `(a + b) << 2`
there, and `1 << 2 + 3` is 32.

T doesn't order them. A bitwise operator or a shift binds more tightly than
a comparison and less than `as`, and mixing two different ones, or one of
them with arithmetic, needs parentheses. A chain of one is fine, as `a | b |
c` means the same either way:

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: u8 = 1;
    let b: u8 = 2;
    let c: u8 = 4;
    out.line(&(a | b | c))?;   // expect: 7
    out.line(&(a & b == 0))?;  // expect: true
    out.line(&((a | b) & c))?; // expect: 0
    Result::Ok(())
}
```

That's a *partial order* of precedence, as Carbon has. It's the same idea
as the rule against chaining comparisons, `a < b < c`, which is already
here: where a reader would have to remember a rule, the program says it.
`t fmt` keeps the parentheses it needs, and adds no others.

## Literals in other bases

`0xFF`, `0b1010_0000` and `0o17`, with `_` between digits and a suffix as
for decimal ones: `0xFFu8`. The prefix is lowercase, `0x`, never `0X`, as
`0O17` would be hard to read. They're the same numbers: `0xFF` is 255, and
must fit in its type as any literal must. `t fmt` keeps a literal as it's
written.

## Wrapping arithmetic, written in T

A hash or a checksum wants arithmetic modulo 2^w, for a type `w` bits wide:
past the greatest value, go on from the least. That's a different
operation from `+`, so it has its own names, in `core::num`:

```rust
use core::num::{wrapping_add, wrapping_mul, wrapping_sub};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&wrapping_add(250u8, 10))?; // expect: 4
    out.line(&wrapping_sub(0u8, 1))?;    // expect: 255
    out.line(&wrapping_add(127i8, 1))?;  // expect: -128
    out.line(&wrapping_mul(100i8, 3))?;  // expect: 44
    Result::Ok(())
}
```

`wrapping_add`, `wrapping_sub` and `wrapping_neg` are written in T, once for
every `T: Int`, and the checker proves they can't panic: `wrapping_add` adds
if the sum fits, and else subtracts 2^w in a way that never leaves the
type, as `a - (greatest - b) - 1 + least`. So they needn't be trusted.
`wrapping_mul` is an intrinsic, as `checked_mul` is: the checker can't
reason about products of unknowns.

There's no truncating conversion, Rust's `300 as u8`, which is 44: masking
and `as` say it, `(x & 0xFF) as u8`, and the checker proves the `as` can't
panic, as `x & 0xFF` is from 0 to 255 whatever `x` is, negative too.

## `Signed` and `Unsigned`

`Int` has two halves: built-in traits that only the signed and only the
unsigned integer types implement, sealed like `Int`, and each with `Int` as
its supertrait. They tell generic code what all their types have in common:

| | the least value | literals |
|---|---|---|
| `T: Int` | at most 0 | 0 to 127 |
| `T: Unsigned` | 0 | 0 to 255 |
| `T: Signed` | -1 minus the greatest | -128 to 127 |

So `core::num::abs<T: Signed>` can be written once, as can `checked_abs` and
`wrapping_abs`. And a `T: Signed` can say `-1`:

```rust
fn minus_one<T: Signed>() -> T { -1 }

fn zero<T: Signed>() -> T { !0 + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&minus_one::<i64>())?; // expect: -1
    out.line(&zero::<i8>())?;       // expect: 0
    Result::Ok(())
}
```

`zero` can't panic: `!0` is -1 in every signed type. In a `T: Int` it isn't:
it's the greatest value of an unsigned one, and `+ 1` doesn't fit.

```rust,error
fn zero<T: Int>() -> T { !0 + 1 }
// error: `+` may panic, but `zero` doesn't say `with panic`
```

## What the logic knows

The logic is linear arithmetic (docs/08-refinement-types.md), and `!x` is
linear, as are `x << k` and `x >> k` for a constant `k`: `x >> 4` is the
`q` with `16·q <= x <= 16·q + 15`. `&`, `|` and `^` aren't linear, nor is a
shift by a variable, so their result is a new value of which the logic
knows some facts, each a lemma of two's complement (`refine::bitwise`):

- `a & b` is from 0 to `a` if `a >= 0`, and the same of `b`; negative if
  both are; and if `b` is all ones, 2^m - 1, then a number from 0 to `b` is
  its own `a & b`.
- `a | b` is at least each of them and at most their sum if both are at
  least 0, and negative if either is negative.
- `a ^ b` is at most their sum if both are at least 0, and at least 0
  exactly when they have the same sign.
- `a >> n` is from 0 to `a`, or from `a` to -1.

That's what proves `utf8_lead` above: `c < 0x800` gives `c >> 6 <= 31`,
which fits in a `u8`, and `31 | 0xC0` is at most `31 + 192`.

## Why it's sound

**Lemma B1 (bitwise results fit).** The values of a `w`-bit signed type
are the integers whose bits from `w - 1` up are all the same, and of an
unsigned one those whose bits from `w` up are all 0. `&`, `|` and `^` work
bit by bit, so if both operands' upper bits are all the same, so are the
result's: it's in the type. So Lemma I1 (every value is of its type) holds
with no panic, which the interpreter checks of every result (`int_of`).

**Lemma B2 (`!`).** `x ↦ lo + hi - x` maps `[lo, hi]` onto itself, and is
the bitwise complement in the type: `-1 - x` for a signed one, the
complement in ℤ, and `hi - x` for an unsigned one, whose bits from `w` up
are 0, so are left alone.

**Lemma B3 (`>>`).** ⌊x / 2^n⌋ is between `min(x, 0)` and `max(x, 0)`, so
it fits in `x`'s type for every `n`.

**Lemma B4 (`<<`).** `x << n` is `x · 2^n`, under the rule for `*`: it
panics unless that fits. By a constant `k` up to 64 it's linear, and proved
like `*`; by more, or a variable, it isn't proved. (Past 64, any nonzero
number of any type doesn't fit; the logic's constants stop short of 2^128,
so it mustn't try.)

**Lemma B5 (the facts).** Each fact above holds of every two integers, so
adding it to what's known keeps "every fact holds" (docs/08), the
induction the logic's soundness rests on. Each has a test that a stronger
version, one that's false, would make the checker accept a wrong program
(`tests/t/err_soundness_bits_*`).

**Lemma B6 (the halves).** In every two's complement type, the greatest is
at least -1 minus the least: equal, in a signed one, and the least is 0 in
an unsigned one. So that's true of every `T: Int`, and the logic knows it,
which is what `wrapping_add` needs. Of a `T: Signed` it's equal, and of a
`T: Unsigned` the least is 0; the literals' ranges are what all of the half
can hold. Like `Int`, both are sealed, so these hold of every instance, now
and in any program (Lemma I4).

The wrapping functions written in T are checked like any code. The one
intrinsic, `wrapping_mul`, is trusted, and its result checked to fit as the
program runs.

## Design decisions worth noticing

- **`!` for both.** Rust's spelling, rather than C's `~`, but decided only
  by the operand's type, never by a guess.
- **Shifts in ℤ.** No arithmetic/logical split, no amount that's too big
  for `>>`, no bits dropped by `<<`.
- **No precedence between unrelated operators.** Parentheses say it.
- **Wrapping is a function, not an operator or a type.** `+%` would be
  three more operators to learn, and a `Wrapping<T>` type needs operator
  overloading, which T doesn't have yet.
- **No truncating conversion.** A mask and `as` say it, and prove it.
- **The shift amount is a `u32`.** A count, not a number of the shifted
  type.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| byte literals, `b'a'` | they need character escapes, which come with strings | 13 ✅ |
| `count_ones`, `leading_zeros`, `rotate_left`, `swap_bytes` | writable in T, with a loop and shifts; intrinsics if they need to be fast | — |
| a truncating conversion `wrap::<u8>(x)` | a mask and `as` do it; generic code may need one | — |
| bitwise operators in refinements | the logic is linear: a predicate couldn't say what `&` is | — |
| `Wrapping<T>`, `Saturating<T>` | need operator overloading | — |

## Exercises

1. Write `fn utf8_len(c: { v: u32 | v <= 0x10FFFF }) -> usize` and
   `fn encode(c: ..., out: &mut [u8])` using `utf8_lead` and `utf8_cont`.
   Which `as` needs a refinement to be proved?
2. Write `count_ones<T: Unsigned>(x: T) -> u32` with a loop. Can it be
   proved not to panic? What does the loop's counter need?
3. Show that `a | b <= a + b` for `a, b >= 0`. Why is it false when `b` is
   negative?
4. Write `wrapping_add` for `u8` only, without `greatest` and `least`. Then
   say why the generic one needs Lemma B6.
5. Change `BinOp::nests_in` so `&` binds more tightly than `|`, as in Rust.
   Which test fails? Write a program whose meaning a reader would get
   wrong without the parentheses.
6. Why can't `x << 64` of a `u64` be `0`, as some machines give?

## Reading

- Warren, *Hacker's Delight* (2nd ed., 2012): the bit tricks, and the
  identities behind B5 (`a + b = (a ^ b) + 2·(a & b)`).
- Knuth, *The Art of Computer Programming*, volume 4A (2011), section
  7.1.3, on bitwise operations and two's complement as 2-adic integers.
- Ritchie, *The Development of the C Language* (1993), on why `&` binds
  less tightly than `==`: it was once `&&`.
- The Carbon language's design, *Operator precedence* (2021): precedence as
  a partial order, with parentheses where it isn't defined.
- Rust RFC 560, *Integer overflow*, on `<<` and `>>` by too much.
