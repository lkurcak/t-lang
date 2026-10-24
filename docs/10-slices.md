# Step 10: slices

Code: `core/slice.t` (`[T]`'s methods, `Iter` and `IterMut`, mostly safe
code over a few intrinsics), `alloc/raw.t` (`RawBuf::slice` and
`slice_mut`), `alloc/list.t` (`as_slice` and `as_mut_slice`), the kinding
rule in `src/check/generics.rs` (`unsized_slice`), `src/check/unify.rs`
(`Clash::Unsized`) and `src/traits.rs` (`matches`), and `src/interp.rs`
(`Interp::slice`).

```rust
use alloc::list::List;

fn total(s: &[i64]) -> i64 with panic {
    let mut sum = 0;
    for x in s.iter() {
        sum = sum + *x;
    }
    sum
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    for i in 1..5 {
        l.push(i);
    }
    for x in l.iter_mut() {
        *x = *x * 10;
    }
    let sum = total(l.as_slice());
    let last = *l.as_slice().last().unwrap();
    let swapped = l.as_mut_slice().swap(0, 3);
    let first = *l.get(0).unwrap();
    out.line(&sum)?;     // expect: 100
    out.line(&last)?;    // expect: 40
    out.line(&swapped)?; // expect: true
    out.line(&first)     // expect: 40
}
```

A *slice*, `[T]`, is some number of `T`s one after another in memory,
seen through a reference: `&[T]` to read them, `&mut [T]` to change them.
How many there are is known only as the program runs, and the reference
carries it, so `s.len()` is a field of the reference, not of what it
points to. `total` takes any run of `i64`s, whoever owns them: a list's,
part of one, or none at all.

## A slice is only behind a reference

`[T]` is a type only right after `&` or `&mut`. A slice is never a value
of its own: not a parameter, a local, a field, a type argument, or what's
in a `Box`. Each of these is an error:

```rust,error
fn f(s: [i64]) {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
```

Nor is `*s` a value, as it would be for a `&i64`: it's only a place, to
borrow again (`&*s`, `&mut *s`) or call a method of.

```rust,error
fn main() {
    let s: &[i64] = core::slice::empty();
    let v = *s;
}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
```

What a type parameter stands for is never a slice either, though the
parameter only shows up behind a reference in the signature. `swap`
exchanges two `T`s; two slices of different lengths can't be exchanged in
place, and the body of a generic function isn't in its signature:

```rust,error
use core::mem::swap;

fn main() {
    let a: &mut [i64] = core::slice::empty_mut();
    let b: &mut [i64] = core::slice::empty_mut();
    swap(a, b);
}
// error: a type parameter can't stand for the slice `[i64]`: it's only ever behind a reference
```

So the rule is one of *kinds*: types split into those a value can have,
which variables and type parameters range over, and slices, which only a
reference can point to. Rust says the same with a `Sized` bound that every
type parameter has unless it says `?Sized`. T has no `?Sized`, so no
generic code ever sees a slice by value, and `&T` for a type parameter
`T` is always a thin reference. The checker keeps the rule in three
places: where a type is written (`resolve`), where a type variable is
solved (`unify` never solves one to a slice), and where an impl is found
(`matches` never lets an impl's parameter stand for one), so `impl<T> Tr
for &T` isn't an impl for `&[i64]`.

In return, `&[T]` and `&mut [T]` are references like any other. A `&'a
[T]` is `Copy` and covariant in `'a` and `T`; a `&'a mut [T]` is
reborrowed when it's used, not `Copy`, covariant in `'a` and invariant in
`T`, for the reason a `&mut T` is (docs/05-ownership.md):

```rust,error
fn shorter<'a, 'b>(s: &'b mut [&'static i64]) -> &'b mut [&'a i64] { s }
// error: lifetime may not live long enough
```

## What a slice can do

A slice's own methods are in `core`, in `impl<T> [T]`, which only the
built-in libraries can write. A few are *intrinsics*, which the checker
trusts:

| Intrinsic | What it does |
|---|---|
| `len(&self)` | how many values it has, a `usize` (docs/11-integers.md) |
| `get(&self, i)`, `get_mut(&mut self, i)` | the value at `i`, or `None` if `i` isn't less than `len()` |
| `split_at(&self, mid)`, `split_at_mut(&mut self, mid)` | the values before `mid` and from it on, or `None` if `mid` is more than `len()` |
| `slice::empty()`, `slice::empty_mut()` | a slice of no values, of any lifetime |

None is `unsafe`, and none panics: each checks the index it's given
against the length the slice carries. The rest, `is_empty`, `first`,
`last`, `swap`, `iter` and `iter_mut`, are safe code over these.

`split_at` and `split_at_mut` give a `Split` or `SplitMut`, a struct of
two slices, `head` and `tail`, with public fields, taken apart by a
pattern: T has no tuples yet.

## Two values at once

Two `get_mut`s of one slice can't both live: the borrow checker doesn't
know whether `i` and `j` are the same index, so it treats them as the
same value.

```rust,error
fn both(s: &mut [i64]) with panic {
    let a = s.get_mut(0).unwrap();
    let b = s.get_mut(1).unwrap();
    *a = *b;
}
// error: cannot borrow `*s` as mutable more than once at a time
```

`split_at_mut` is how to: it uses the `&'a mut [T]` up and gives two
`&'a mut [T]`s, of values before `mid` and from it on, which share none.
That's the one fact about memory a slice's safe code needs: a unique
borrow of a run of values can be split into unique borrows of its parts.
`swap(i, j)` is `split_at_mut` between the two, then `get_mut` of each
half, then `core::mem::swap`.

```rust
use core::slice::SplitMut;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(1);
    l.push(2);
    let SplitMut { head, tail } = l.as_mut_slice().split_at_mut(1).unwrap();
    let a = head.get_mut(0).unwrap();
    let b = tail.get_mut(0).unwrap();
    *a = *a + *b;
    *b = 0;
    let first = *l.get(0).unwrap();
    let second = *l.get(1).unwrap();
    out.line(&first)?; // expect: 3
    out.line(&second)  // expect: 0
}
```

## `iter_mut` is safe code

An iterator of `&'a mut T`s must give references that outlive each call
of `next`, which only borrows the iterator, and no two of them may be the
same value. Step 9.1 couldn't write one (docs/09.1-list.md): from a
`&'b mut IterMut<'a, T>`, safe code only gets a `&'b mut T`. With
`split_at_mut`, it can:

```text
pub struct IterMut<'a, T> { rest: &'a mut [T] }

fn next(&mut self) -> Option<&'a mut T> {
    let rest: &'a mut [T] = mem::replace(&mut self.rest, empty_mut());
    match rest.split_at_mut(1) {
        Option::Some(SplitMut { head, tail }) => {
            self.rest = tail;
            head.get_mut(0)
        }
        Option::None => Option::None,
    }
}
```

It takes what's left out of the iterator, leaving an empty slice, so it
holds the `&'a mut [T]` itself, not a reborrow through `self`. It splits
off the first value and keeps the tail. The value it gives is in no slice
it holds any more, so nothing it gives later can be that value too. Each
reference borrows the slice, for `'a`:

```rust,error
use alloc::list::List;

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    l.push(1);
    let s = l.as_mut_slice();
    let mut it = s.iter_mut();
    let a = it.next().unwrap();
    let _v = *s.get(0).unwrap();
    *a = 2;
}
// error: cannot borrow `*s` while `*s` is mutably borrowed
```

`iter` is the same with `split_at`, and `Copy` slices.

## Slices of a list

A slice is made only by `alloc`, out of a `RawBuf`, by two new unsafe
operations, which replace `RawBuf::get` and `get_mut`:

- `slice(&self, lo, hi) -> &[T]` and `slice_mut(&mut self, lo, hi) -> &mut
  [T]`: the values in slots `lo..hi`. Their obligation is `lo <= hi <=
  cap()`, and that slots `lo..hi` are initialised. `lo >= 0` is the
  type's, `usize`, as an index's is.

The slice borrows the buffer, so while it lives nothing can `write`,
`read`, `grow` or `free` it, and the slots stay initialised: nothing done
through a `&mut [T]` takes a value out, as no safe code can move out of a
reference. So the obligation is only checked once, when the slice is
made, and the interpreter does check it:

```rust,error
use alloc::raw::RawBuf;

fn main() with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(2);
    // SAFETY: slot 0 is uninitialised.
    unsafe { b.write(0, 1) };
    // SAFETY: wrong: slot 1 holds nothing.
    let s = unsafe { b.slice(0, 2) };
    let _v = s.len();
    // SAFETY: slot 0 holds the 1; then every slot is uninitialised.
    unsafe {
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::slice` of slots 0..2, but slot 1 is uninitialised
```

`List::as_slice` is `buf.slice(0, len)`, whose obligation is the list's
invariant (docs/09.1-list.md), and `as_mut_slice` the same with
`slice_mut`. Then `get`, `get_mut`, `iter`, `iter_mut` and `swap` are
the slice's: `List`'s `unsafe` blocks are now only in `push`, `pop`,
`free_with` and these two. The slice borrows the list, so `push`, which
may move the values, can't run while it lives:

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut l: List<i64> = List::new();
    l.push(1);
    let s = l.as_slice();
    l.push(2);
    let _v = s.len();
}
// error: cannot borrow `l` as mutable while `l` is borrowed
```

A slice of a list of values that aren't `Discard` is fine: it only
borrows them, and nothing safe can overwrite one without giving the old
value back (`core::mem::replace`).

## No indexing yet

There's no `s[i]`:

```text
fn main() {
    let s: &[i64] = core::slice::empty();
    let x = s[0];
}
// syntax error: there's no indexing with `[..]` yet; use `.get(..)`, which gives an `Option`
```

Rust's `s[i]` panics when `i` is out of range. The principled `s[i]` is
one the checker proves in range, `0 <= i < len(s)`, as it proves
arithmetic total (docs/08-refinement-types.md), so it needs no check and
can't fail. That needs refinements that can name `len(s)`, a *measure*,
which T doesn't have yet. A slice is where a measure is easiest: its
length never changes, unlike a list's, so `len(s)` of a variable `s`
stays true for as long as `s` does. Until then, `get` returns an
`Option`, and a program that knows better says so with `unwrap`, in its
type.

## Why it's sound

> A program whose `unsafe` blocks keep their obligations, using slices
> only through their methods, doesn't go wrong.

A `&'a [T]` from `RawBuf::slice` is a shared loan of slots `lo..lo+len`,
each holding a value, for `'a`; a `&'a mut [T]` a unique one. The
lemmas:

- **Kinding.** No value has a slice type, so nothing moves, copies, drops
  or assigns a slice. Only places do: `*s`, under `&`, `&mut` or a method
  call. A type parameter never stands for a slice, so the types generic
  code moves around are all of values. That's what makes `swap` and every
  other generic function sound for slices without a `?Sized` in sight:
  they never see one.
- **The intrinsics.** Each is total and checks its index against the
  length the slice carries, so it only ever gives a reference to a slot
  in its range, which holds a value by the loan. `split_at_mut` uses its
  loan up and gives two of disjoint ranges, together the original (the
  split lemma of RustBelt's slices); `empty_mut` a loan of no slots,
  which aliases nothing.
- **Loans.** A slice's methods borrow it as `&self` or `&mut self`, and
  each result has the lifetime of that borrow, so the borrow checker's
  rules for references are the rules for slices: no `&mut` loan is used
  while another of the same values lives, and none outlives the buffer,
  which `grow` and `free` need by `&mut` or by value.
- **Safe code over them.** `swap`, `iter`, `iter_mut` and the rest are
  checked like any program, so they can't break anything the intrinsics
  don't.

So the trust is the intrinsics, which are a few lines each in
`Interp::slice`, and `RawBuf::slice`'s obligation, which the interpreter
checks as the program runs.

## Design decisions worth noticing

- **`[T]` is a kind of its own, not a type with a `Sized` bound.** Rust's
  `?Sized` lets generic code take `&T` for an unsized `T`; T has nothing
  that needs it yet, and each `?Sized` function is one more to get right.
  Kept out, generic code is all sized, and `&[T]` is just a reference.
- **The type parameter rule is in unification, not in signatures.**
  Looking at whether `T` is only behind references in a signature isn't
  enough: the body can do anything with a `T`, so the rule is about what
  a type parameter stands for, wherever it's decided.
- **One fact about memory, `split_at_mut`.** `len`, `get`, `get_mut` and
  `split_at` are trusted too, but each is a simpler case of the same
  loan; `iter_mut` and `swap` are safe code. A new way to split a borrow
  is one more intrinsic to check, not a change to the borrow checker.
- **Total operations.** No intrinsic panics or has an obligation: a
  slice knows its length, so checking it is cheap, and the caller decides
  what an index out of range means.
- **`RawBuf` hands out slices, not single slots.** `slice(i, i + 1)` is
  `get(i)`, so `RawBuf::get` and `get_mut` went: the trusted base got
  smaller, and every reference into a buffer is through a slice.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `s[i]` | it should be proved in range, with no panic: refinements that name `len(s)` (measures) | — |
| ranges, `s.slice(lo, hi)`, `&s[lo..hi]` | `split_at` twice does it; a method of its own when it's wanted | — |
| `==` on slices | element by element, through `Eq` of `T`: a built-in impl to write | 13 ✅ |
| `str` | a slice of bytes that are UTF-8 | 13 ✅ |
| `Box<[T]>`, `Box<str>` | values whose size is known only as the program runs, owned | — |
| arrays, `[T; N]` | a length in the type: const generics | — |
| `for x in s` | `IntoIterator`, which still needs negative reasoning (docs/09.1-list.md) | — |
| tuples | `Split` and `SplitMut` are structs until there are | — |

## Exercises

1. Write `first_mut` and `last_mut` for `&mut [T]`, as safe code. Then
   `split_first_mut`, which gives the first value and the rest.
2. Write `reverse(&mut self)` with `swap`. Then without it, with
   `split_at_mut`: why does the second need no `unsafe` either?
3. Make `split_at_mut` give `tail` from `mid - 1` on (in `Interp::slice`).
   Which test finds it? Which safe function goes wrong first?
4. Take `Type::Slice(_) => return Err(Clash::Unsized(v, t))` out of
   `unify`. Write a program that then swaps two slices of different
   lengths, and see what the interpreter says.
5. Why is `split_at` (not `_mut`) an intrinsic, when `get` could do what
   it does one value at a time? What would `Iter` cost then?
6. `RawBuf::slice` checks its obligation only when the slice is made. What
   would have to change for it to need checking again later?

## Reading

- Jung, Jourdan, Krebbers and Dreyer, *RustBelt: Securing the Foundations
  of the Rust Programming Language* (2018): slices, and `split_at_mut`'s
  splitting of a borrow, proved sound.
- *The Rustonomicon*, "Splitting Borrows": why `iter_mut` needs
  `split_at_mut`, and how it's written.
- Matsakis, "DST, Take 5" (2014), and the Rust reference on dynamically
  sized types: `?Sized`, which this step does without.
- Xi and Pfenning, *Eliminating Array Bound Checking Through Dependent
  Types* (1998), and Vazou, Seidel, Jhala, Vytiniotis and Peyton Jones,
  *Refinement Types for Haskell* (2014), on measures: the `s[i]` to come.
