# Step 11: integer types

Code: `src/types.rs` (`IntTy`), the typing rules in `src/check/expr.rs`
(T-Int, T-Arith, T-As) and `src/check/unify.rs` (`fresh_int`), whether
literals fit in `src/check/ints.rs`, the ranges in the refinement logic in
`src/check/refine.rs` (`Walk::range`) and `src/check/refine/logic.rs`
(`Kind`), exhaustiveness in `src/exhaust.rs`, the built-in trait `Int` in
`src/traits.rs`, `core/num.t`, and `src/interp.rs` (`Value::Int`,
`Interp::arith`).

```rust
fn mean(a: u8, b: u8) -> u8 {
    // Each is at most 255, so their sum fits in a `u16`.
    let sum = a as u16 + b as u16;
    (sum / 2) as u8
}

fn main() {
    print(mean(200, 255)); // expect: 227
    let big: u64 = 18446744073709551615;
    print(big); // expect: 18446744073709551615
    for i in 0..3u8 {
        print(i); // expect: 0
    }             // expect: 1
                  // expect: 2
}
```

Until now there was one integer type, `int`, which was an `i64` as the
program ran. Now there are ten, and no `int`:

| | 8 bits | 16 | 32 | 64 | the machine's |
|---|---|---|---|---|---|
| signed | `i8` | `i16` | `i32` | `i64` | `isize` |
| unsigned | `u8` | `u16` | `u32` | `u64` | `usize` |

Each is a set of integers, from its least to its greatest: a `u8` is one of
0 to 255. Nothing in `mean` can panic, and the checker proves it: the sum
of two numbers of at most 255 is at most 510, which is a `u16`; half of
that is at most 255, which is a `u8`. So `mean` needn't say `with panic`.

## One rule for every operator

```text
  Γ ⊢ l : τ    Γ ⊢ r : τ    τ: Int
  --------------------------------- (T-Arith)   for + - * / %
          Γ ⊢ l + r : τ
```

Both sides have the same type, and so does the result. Nothing is widened
for you, as C does by its *usual arithmetic conversions*, which turn an
`unsigned` comparison with `-1` into one with 4294967295.

```rust,error
fn f(a: u8, b: u16) -> u16 with panic { a + b }
// error: expected `u8`, found `u16`
```

What an operator means is what it means in the integers, ℤ, if that fits
in its type, and a panic if it doesn't. It never *wraps*: `255u8 + 1` is a
panic, always, where Rust gives 0 in a release build and panics only in a
debug one. Unary `-` is the same rule: `-x` of a `u8` fits only when `x` is
0, and of an `i8` unless `x` is -128, whose negation is one too big.

So every integer value is one of its type's, and an operator's result is
the same number whatever the type is, if it has one. That's what lets the
refinement logic, which is about ℤ, say what an operator does: it's the
same as in the logic, or the program has stopped. Since this step that
includes dividing by a positive constant `k`, which is linear: rounding
towards zero, `a / k` is the `q` with `k·q <= a <= k·q + k - 1` when `a >=
0` (and the mirror of that when `a <= 0`), and `a % k` is less than `k`
from 0, with the sign of `a`. That's how the checker knows, in `mean`,
that half of 510 is at most 255.

## Literals: no default

A literal's type is worked out from how it's used, like any type the
program doesn't write (docs/03-inference.md), and nothing else:

```rust,error
fn main() {
    let x = 5;
    print_bool(x == 5);
}
// error: cannot infer the type of this integer
```

Rust would make `x` an `i32` here, and Haskell an `Integer`: a *default*,
a type picked for you by a rule of the language. Which type it is changes
what the program does, where it overflows, so T says nothing it can't
take back: a default can be added later without breaking any program that
checks now, but not taken away. A suffix says which in place:

```rust
fn main() {
    print_bool(1i64 > 2); // expect: false
    print(1000u32 * 3);   // expect: 3000
}
```

A literal must fit in its type, which is checked once that's known
(`Checker::literals`): `let x: u8 = 256;` is an error, not a panic. A `-`
right before a literal is part of it, so `-128` is an `i8`, though `128`
isn't. Of a `usize` or `isize`, it must fit on every machine, so in 32 bits;
of a type parameter, in every integer type (below).

## `as`

```text
   Γ ⊢ e : σ    σ: Int    τ: Int
  ------------------------------- (T-As)
         Γ ⊢ e as τ : τ
```

`x as u16` is the same number, as a `u16`, and panics if it isn't one: it's
an operator like `+`, under the same rule, so it can't panic where the
checker proves the number fits, which it always does from a type into one
that holds all of its values. Rust's `as` truncates instead: `300 as u8` is
44. Here it's never anything but the same number, so a conversion never
loses one silently:

```rust
fn byte(n: i64) -> Option<u8> {
    if n >= 0 && n < 256 { Option::Some(n as u8) } else { Option::None }
}

fn main() {
    print(byte(255).unwrap_or(0)); // expect: 255
    print(byte(256).unwrap_or(0)); // expect: 0
}
```

`byte` can't panic: the `if` says `n` is a `u8`'s, and the checker knows.
That's how a conversion that can fail is written, in T, with nothing new to
trust.

What follows `as` is a type's name, as only an integer type or a type
parameter can be there. So `x as u8 < y` compares, rather than starting
`u8`'s type arguments, which is a well-known trap in Rust's grammar.

## `usize` is as big as the machine says

Lengths and indices are `usize`s: a slice's `len()`, `get(i)`, a `List`'s.
How big a `usize` is depends on the machine, 32 or 64 bits, and the
checker doesn't pick one. In its logic, the greatest `usize` is a variable,
which all it knows of is that it's between the greatest 32-bit and the
greatest 64-bit number (`Walk::range`). So what it proves holds on both:

```rust
fn index(i: u32) -> usize { i as usize }

fn wide(n: usize) -> u64 { n as u64 }

fn main() {
    print(index(7)); // expect: 7
    print(wide(8));  // expect: 8
}
```

```rust,error
fn f(n: u64) -> usize { n as usize }
// error: `as` may panic, but `f` doesn't say `with panic`
```

A `u64` fits in a 64-bit `usize`, and the interpreter is one, but not in a
32-bit one. Fixing the size at 64 would have been simpler, but not a
restriction to relax later: every proof that counted on it would break on
the day a 32-bit target came. This way a program is checked once for every
machine. `isize` is the same, with its least the negation of its greatest,
minus one, and the greatest `usize` twice the greatest `isize`, plus one:
both are written with the same variable, so the logic knows how they're
related.

A `match` must cover a `usize` on every machine too, so it's checked
against 64 bits, which covers 32.

## Generic code: `T: Int`

```rust
fn double<T: Int>(x: T) -> T with panic { x + x }

fn succ<T: Int>(x: T) -> T {
    if x < core::num::greatest::<T>() { x + 1 } else { x }
}

fn main() with panic {
    print(double(100u8)); // expect: 200
    print(double(-3i32)); // expect: -6
    print(succ(255u8));   // expect: 255
}
```

`Int` is a built-in trait that the ten integer types implement, and no
impl can add to: it's *sealed*. A `T: Int` may be any of them, so generic
code over it can count on what they all have in common, and that's all:
the operators and comparisons work on it, it's `Copy` and `Eq`
(`trait Int: Copy + Eq`), and a literal of type `T` must fit in every
integer type, so be between 0 and 127. In the logic, `T`'s least and
greatest are two variables, with the least at most 0 and the greatest at
least 127; `core::num::least::<T>()` and `greatest::<T>()` are what they
are. `succ` checks: `x` is less than the greatest, so `x + 1` fits, in
whichever type it is.

What isn't common isn't known:

```rust,error
fn abs<T: Int>(x: T) -> T {
    if x < 0 { 0 - x } else { x }
}
// error: `-` may panic, but `abs` doesn't say `with panic`
```

That's right for every unsigned type, where `x < 0` never holds, and wrong
for `i8`, where `0 - -128` doesn't fit. A proof of generic code must hold
of every instance, and the checker can't tell which it is.

That a trait is closed is what makes this sound. If a program could add
`impl Int for Meters`, generic code that counted on "at most 0 and at
least 127" could be called at a type of which that isn't true. A closed
set of instances is what an ML datatype is to its constructors, and what
makes its `match` exhaustive.

`core` is written over `Int` once for all ten: `checked_add`,
`checked_sub`, `checked_div`, `checked_rem` and `checked_neg` (each proved
not to panic), the intrinsic `checked_mul`, `Range<T>`'s iterator, and the
`Ord` impls through a generic `compare`. `checked_div` can't write `-1`, a
literal that doesn't fit in a `u8`, so it says `b < 0 && b + 1 == 0`.

## Why it's sound

**Lemma I1 (every value is of its type).** Each type `τ` is the integers
from `lo(τ)` to `hi(τ)`. A literal is checked to fit, statically (on every
machine). Every operator and `as` gives its result in ℤ if that's between
`lo(τ)` and `hi(τ)`, and panics otherwise, an effect its function must say
unless the checker proved it can't happen. So by induction on evaluation
every value of type `τ` is one of `τ`'s, which the interpreter checks of
every integer it makes (`int_of`).

**Lemma I2 (the logic's ranges are true).** The logic gives each variable
of type `τ` the fact `lo(τ) <= v <= hi(τ)` (`logic::impossible`). For a
fixed type these are constants, and true by I1. For `usize` and `isize`,
and a type parameter, they're sums of *bound* variables, each between two
constants. A proof holds for every value of those that the constants
allow, so for each machine (32 or 64 bits) and for each integer type a
type parameter can be, which are all between the constants: an `i8`'s
least is at most 0, a `u64`'s greatest at most `2^64 - 1`. So what the
checker proves holds of every instance, whichever the program runs at.

**Lemma I3 (division).** `a / b` and `a % b` don't fit only when `b` is 0,
or when `a` is the least value of a signed type and `b` is -1: then the
quotient is one past the greatest. Every integer type is a two's
complement one, with its least at most 0 and the greatest at least
`-1 - least`, and otherwise the quotient is no further from 0 than `a`. So
the checker's goal, "not by 0, nor the least by -1", is enough in every
integer type, and so of a type parameter's, though the logic doesn't know
which type it is.

**Lemma I4 (`Int` is closed).** Only the ten integer types implement
`Int`: it's built in, and an impl of it is an error. So I2's constants hold
of every `T: Int`, now and in any program.

Literals are typed by unification, with no default (D3). An integer type
that's never decided is an error before anything relies on it
(`Checker::solve_all`), so every value has one of the ten types, or a type
parameter's, by the time the program runs. Monomorphisation writes each
literal's type into it, and the interpreter's values carry theirs: adding
a `u8` to a `u16` there is a checker bug, which it reports.

## Design decisions worth noticing

- **No default type.** It's the conservative choice: one can be added
  without breaking a program, not removed. Suffixes say a literal's type
  where nothing else does.
- **No wrapping.** Arithmetic modulo `2^n` is a different operation, and
  should look like one: `wrapping_add` and friends come with bytes and
  strings, where they're first needed.
- **Unary `-` on unsigned types.** Rust rejects `-x` of a `u8`; here it's
  the same rule as every other operator, and fits only at 0. A lint can
  say it's probably a mistake.
- **`as` is checked.** It can't change a number, so it can't lose one.
  Truncating, as Rust's `as` does, is a different operation too.
- **The size of `usize` is unknown.** Proofs are of every machine; only the
  run is of one.
- **Ten types, one trait.** `Int` lets `core` be written once, rather than
  ten near-copies of each function.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `i128`, `u128` | the logic's constants are `i128`s, and `u128`'s greatest isn't one | — |
| wrapping and truncating arithmetic, bitwise operators, shifts | for bytes and strings, where they're first needed | — |
| a default type for literals | can be added; can't be taken back | — |
| `T: Signed`, `T: Unsigned` | `Int`'s two halves, for `abs` over signed types | — |
| `From`/`TryFrom` | `as` and an `if` do it; traits for conversions belong to `std` | — |
| refinements of a type parameter, `{ x: T \| x > 0 }` | refinements of integer types only, as before | — |

## Exercises

1. Write `fn clamp_u8(n: i64) -> u8` without `with panic`.
2. `fn f(n: usize) -> usize { n + 1 }` may panic. Write a refinement on
   `n` that makes it not, and that every `u32` meets.
3. Write `checked_abs<T: Int>(x: T) -> Option<T>` with no panic. Which
   facts about `least::<T>()` does it need?
4. Change `Walk::range` so a `usize` is 64 bits in the logic. Which test
   fails, and what would a 32-bit target do with the program it rejects?
5. Why must a literal of a type parameter be at most 127, and not 255?
6. Let users write `impl Int for Meters`. Write a program that the
   checker then accepts and that goes wrong.

## Reading

- Pierce, *Types and Programming Languages* (2002), chapter 15, on base
  types and subtyping, which numeric widening would be and isn't here.
- Dietz, Li, Regehr and Adve, *Understanding Integer Overflow in C/C++*
  (2012): how often wrapping arithmetic is a bug.
- The Rust RFC 560, *Integer overflow*: why Rust panics in debug builds
  and wraps in release ones, which T doesn't.
- Hindley–Milner with type classes and defaulting: the Haskell 2010
  report, section 4.3.4, on what a default does and why it's a special
  rule.
- Xi and Pfenning, *Dependent Types in Practical Programming* (1999):
  integers in types, which refinements bring here.
