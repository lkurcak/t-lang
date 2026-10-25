# Step 22: arrays

Code: `core/array.t` (`[T; N]`'s methods, mostly safe code over three
intrinsics, and `from_fn`), `src/types.rs` (`Type::Array`, `Type::Nat`,
`Params`' const parameters), `src/check/generics.rs` (`len`, the kinds of
type arguments), `src/check/expr.rs` (T-Array, T-Repeat),
`src/check/patterns.rs` (array patterns), `src/check/refine.rs`
(`Val::Array`, `Walk::length`, `PredName::Const`), `src/mono.rs` (a const
parameter as a value) and `src/interp.rs` (an array's `Slots`).

Until now, a run of values one after another had to be on the heap, in a
`List`, or somewhere else seen through a slice. An *array*, `[T; N]`, is
exactly `N` values of type `T`, inline: in a local, a field, or another
array, with no allocation and no pointer to follow.

```rust
fn total(s: &[i64]) -> i64 {
    let mut sum: i64 = 0;
    for x in s.iter() {
        sum = core::num::wrapping_add(sum, *x);
    }
    sum
}

#[derive(Copy, Discard)]
struct Rgb { c: [u8; 3] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = [1, 2, 3, 4];
    a[0] = 10;
    out.line(&a[0])?;                // expect: 10
    out.line(&a.len())?;             // expect: 4
    out.line(&total(a.as_slice()))?; // expect: 19
    let z: [i64; 3] = [7; 3];
    out.line(&total(z.as_slice()))?; // expect: 21
    let p = Rgb { c: [255, 128, 0] };
    out.line(&p.c[1])?; // expect: 128
    Result::Ok(())
}
```

`[1, 2, 3, 4]` lists its values, so its type is `[i64; 4]`; `[7; 3]` is
three copies of `7`. `a[0]` is `*a.index(0)`, as for a slice, proved in
range from `0 < 4` (docs/21-measures.md), so nothing here can panic, and
an array isn't a slice: `as_slice()` makes one.

## The length is in the type

An array's length is part of its type: `[i64; 3]` and `[i64; 2]` are as
different as `i64` and `bool`, so a value of one can't be used as the
other.

```rust,error
fn main() {
    let _a: [i64; 3] = [1, 2];
}
// error: expected `[i64; 3]`, found `[i64; 2]`
```

That's what makes `len()` known everywhere: it's `N`, for every value of
the type, wherever it's asked, unlike a slice's, which is the reference's
and only known as the program runs.

> **Lemma A1.** Every value of `[T; N]` has exactly `N` values in it.

A literal of `k` values has type `[T; k]`; `[v; n]` has `n`; `from_fn`
makes as many as its result's type says. Nothing changes how many there
are: there's no `push`, and `index_mut` writes over one. The interpreter
checks it, as an array's values are kept as exactly that many slots.

## Const parameters

A function that takes an array of any length is generic over it, with a
*const parameter*: `const N: usize` stands for a length the caller picks,
as a type parameter stands for a type. It's worked out from the
arguments, or written, and in the body it's a `usize`, the same all
through the call.

```rust
fn sum<const N: usize>(a: &[i64; N]) -> i64 {
    let mut s: i64 = 0;
    let mut i: usize = 0;
    while i < N {
        s = core::num::wrapping_add(s, a[i]);
        i = i + 1;
    }
    s
}

fn zeros<const N: usize>() -> [i64; N] { [0; N] }

/// The last `N` values pushed.
#[derive(Discard)]
struct Ring<T, const N: usize> { buf: [T; N], head: { h: usize | h < N } }

impl<T: Copy, const N: usize> Ring<T, N> {
    fn push(&mut self, v: T) {
        self.buf[self.head] = v;
        let next = self.head + 1;
        self.head = if next < N { next } else { 0 };
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&sum(&[1, 2, 3]))?;    // expect: 6
    out.line(&sum(&[5; 10]))?;      // expect: 50
    out.line(&zeros::<3>().len())?; // expect: 3
    let mut r: Ring<i64, 2> = Ring { buf: [0, 0], head: 0 };
    r.push(1);
    r.push(2);
    r.push(3);
    out.line(&r.buf[0])?; // expect: 3
    Result::Ok(())
}
```

`Ring`'s `head` is always an index of `buf`: its refinement names `N`,
which is how long `buf` is, so `self.buf[self.head]` is proved in range,
and `push` keeps the invariant, as `next < N` says.

A const parameter is a parameter of a kind of its own, as an effect
parameter is (docs/17-retrospective-1.md): the declaration says which, and
using one as the other is an error, never a matter of spelling.

```rust,error
fn f<T>(a: [i64; T]) {}
// error: `T` is a type parameter, not a const; declare it as `const T: usize`
```

```rust,error
fn zeros<const N: usize>() -> [i64; N] { [0; N] }

fn main() {
    let _z = zeros::<i64>();
}
// error: this argument of `zeros` is a length, not a type
```

Its name is upper case, `N` or `CAP`: it's a constant, as a global will
be, and the `non_upper_case_globals` lint says so.

> **Lemma A2.** Two lengths are equal only if they're the same `Nat`, or
> the same const parameter, or one is a variable that becomes the other.

There's no arithmetic on lengths, no `[T; N + 1]`, so equality is
syntactic, and unification stays the first-order unification it was
(docs/03-inference.md): decidable, with most general solutions.
Monomorphisation replaces each const parameter by the `Nat` it is in the
copy, and polymorphic recursion can't grow a length, as a type argument
for one is only ever a literal or a parameter.

## Making arrays

`[a, b, c]` is an array of what it lists, all of one type. `[v; n]` is
`n` copies of `v`, so `v` must be `Copy`; and `n` must be known before the
program runs, as it's the array's length: an integer literal or a const
parameter.

```rust,error
fn main() {
    let n: usize = 3;
    let _a = [0; n];
}
// error: an array's length must be an integer literal or a const parameter
```

An array of values that aren't `Copy`, a list in each place say, is made
by `core::array::from_fn`, which calls its closure with each index in
turn, and does what that does:

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let squares: [u64; 4] = core::array::from_fn(|i| i as u64 * i as u64);
    out.line(&squares[3])?; // expect: 9
    let lists: [List<i64>; 2] = core::array::from_fn(|_| List::new());
    out.line(&lists[1].len())?; // expect: 0
    Result::Ok(())
}
```

## Indexing

`a.len()` is a measure, as a slice's is, and an intrinsic: its value is
`N`. So `index` and `index_mut` take `i: { i: usize | i < self.len() }`,
and `a[i]` needs that proved. In generic code `N` is a variable of the
logic, from 0 to `usize::MAX`, of which only what the program learns is
known:

```rust,error
fn first<const N: usize>(a: &[i64; N]) -> i64 { a[0] }
// error: can't prove `i < self.len()` for argument `i` of `index`
```

`N` may be 0. A test says what's needed: in `if N > 0 { a[0] } else { 0 }`
the checker knows `a.len() > 0`, as `a.len()` and `N` are the same
variable.

The logic follows an array as its length: `Val::Array(n)`, made from its
type wherever a value of it appears, and kept behind any reference to it,
which can't change it (A1). What its values are isn't followed.

## Taking one apart

A pattern `[a, b, c]` matches an array of exactly three values, each by
the pattern in its place, and moves them out, as a struct pattern does.
That's how an array of values that must be used up is used up, one by one:

```rust
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn first_true(a: [bool; 2]) -> i64 {
    match a {
        [true, _] => 0,
        [false, true] => 1,
        [false, false] => 2,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let [a, b] = [Fd { n: 3 }, Fd { n: 4 }];
    let y = close(b);
    let x = close(a);
    out.line(&(x + y))?;                   // expect: 7
    out.line(&first_true([false, true]))?; // expect: 1
    Result::Ok(())
}
```

> **Lemma A6.** An array pattern of `k` values only matches a `[T; k]`.

It's checked against the array's type, so a length that's a const
parameter isn't one, as it could be any:

```rust,error
fn one<const N: usize>(a: [i64; N]) -> i64 {
    let [x] = a;
    x
}
// error: this pattern is for an array of 1, but the value is `[i64; N]`
```

And so a `match` must cover each combination of the values, as for a
struct with that many fields, by the same exhaustiveness checking
(docs/02-adts.md). There's no `[first, ..]` yet.

## What an array is

An array is its values, so it's whatever they all are:

> **Lemma A5.** `[T; N]` is `Discard`, `Copy` or `Eq` exactly when `T` is.

Dropping one drops each value, front to back (docs/20-destructors.md), so
an array of values that must be used up must be used up too; copying one
copies them, which `Copy` allows; `==` compares them index by index. A
derived impl of a struct that holds a `[T; N]` needs what it needs of
`T`. The borrow checker sees an array as its values too: a `[&'a i64; 2]`
is covariant in `'a`, as a `&'a i64` is, a `&mut [T; N]` invariant in
`T`, and a `&'a [&'b i64; 1]` a type only if `'b` outlives `'a`.

```rust,error
struct Fd { n: i64 }

fn main() {
    let _fds = [Fd { n: 1 }, Fd { n: 2 }];
}
// error: `_fds` is never used up, and its type, `[Fd; 2]`, isn't `Discard`
```

## Why it's sound

> A value of `[T; N]` is `N` values of type `T`; every length the checker
> knows of is the one the value has; and every index it proved is less
> than it.

A1 and A2 say the type tells the length: a value is made with as many
values as its type says, nothing changes how many, and a value can only
be used at an equal type, whose length is the same. So `len()`, which is
the type's length, is the value's, which is what the logic says of
`Val::Array` (A3, the measure's lemma M1 of docs/21-measures.md for an
array); in generic code, of the variable for `N`, which is one value for
the whole call, as the caller picked it. Indexing is then step 21's
argument: `index`'s refinement holds, so the intrinsic can't fail. Kinds
(A4) keep a length from standing where a type is, and back. A5 and A6 are
the ownership argument of structs (docs/05-ownership.md) for `N` fields,
which an array pattern names by position.

The interpreter checks it as it runs: an array is exactly its slots, an
array pattern of another length is "type soundness violated", and so is
an index out of range, as for slices; the intrinsics' refinements are
checked, `as_slice`'s `s.len() == self.len()` included; and a refinement
naming a const parameter is checked with the length the copy has.

## Design decisions worth noticing

- **Const parameters, now.** Without them no code could be generic over
  a length, and `Ring<T, N>` couldn't be written; they cost little, as a
  kind of type parameter, as effect parameters are.
- **No arithmetic on lengths.** `[T; N + 1]` would make type equality
  arithmetic, unification modulo a theory, and polymorphic recursion able
  to grow a length without end. Syntactic equality is the restriction
  that keeps the old arguments as they were.
- **No coercion to a slice.** `total(&a)` doesn't type; `as_slice()` says
  it. T has no implicit coercions, and inference stays simpler for it.
- **`N` is a value too.** In bodies and in refinements, so a struct can
  say what an index of its array is without an array to measure.
- **Only `usize`.** A const parameter is a length; its type is written,
  so others can come later without breaking any.
- **Patterns, of an exact length.** The one way to take apart an array of
  linear values; `..` waits for a way to say what's left.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `[first, .., last]`, `[x, rest @ ..]` | a pattern whose rest is an array of a length computed from another | — |
| `[T; N + 1]`, `[T; 2 * N]` | arithmetic on lengths, so type equality modulo it | — |
| refined const parameters, `const N: { n: usize \| n > 0 }` | what an instantiation must prove, at each use | — |
| `fn cap(&self) -> usize { N }` as a measure | a measure's body that names a const parameter, instantiated at each call | — |
| `&[T; N]` used as a `&[T]` | an implicit coercion, against inference's grain | — |
| an array's values, by index, in the logic | the theory of arrays, `select` and `store` | — |
| `map`, consuming an array value by value | moving out of each slot in turn, with what's left dropped if `f` panics | — |

## Exercises

1. Write `fn reverse<const N: usize>(a: &mut [i64; N])` with `a.swap` and
   no `with panic`. What does each index need proved?
2. Why must `[v; n]` need `Copy` rather than `Discard`? Write what goes
   wrong with `[Box::new(1); 2]`.
3. Make `Ring` hold values that aren't `Copy`. What does `push` do with
   the value it writes over, and what does `new` start the buffer as?
4. Add `[T; N + 1]` by hand: what does unifying `[i64; N + 1]` with
   `[i64; 3]` have to solve, and with `[i64; M + 1]`?
5. Why is a length that's a const parameter not matched by `[x]`, even in
   a copy where it's 1? What would checking each copy cost?
6. Follow an array's values in the logic, as `select(a, i)` and
   `store(a, i, v)`. What would `a[0] = 1; a[0] == 1` need?

## Reading

- Hongwei Xi and Frank Pfenning, *Dependent Types in Practical
  Programming* (1999): indices in types, lengths among them, decided by
  constraint solving, as DML does.
- Hongwei Xi, *Applied Type System* (ATS): arrays whose lengths are in
  their types, and proofs of the indices.
- The Rust RFC 2000, *Const generics* (2017): const parameters, and why
  arithmetic on them was left out.
- John McCarthy, *Towards a Mathematical Science of Computation* (1962):
  the theory of arrays, `select` and `store`.
- Aaron R. Bradley, Zohar Manna and Henny B. Sipma, *What's Decidable
  About Arrays?* (2006): which properties of arrays a solver can decide.
