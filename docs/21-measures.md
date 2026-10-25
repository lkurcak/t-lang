# Step 21: measures

Code: `src/check/refine.rs` (`Measure`, `find_measure`, `measure_body`,
`Walk::measure`, the objects in `Val`), `src/check/modules.rs` (`x[i]`),
`src/borrowck/build.rs` (`calls_in`), `src/interp.rs` (`Interp::measure`),
`core/slice.t` and `core/str.t` (`len`, `index`), `alloc/raw.t`,
`alloc/list.t` and `alloc/string.t`.

Since step 8 a refinement can say what an integer is, but only in terms
of other integers and `bool`s. So it couldn't say "`i` is an index of
`s`", which needs how long `s` is, and `s[i]` had to wait (docs/10-slices.md):
a slice's values were got with `get`, an `Option`, and a program that knew
`i` was in range said so with `unwrap`, `with panic` in its type. Now a
method can be a *measure*, which a refinement can name:

```rust
fn sum(s: &[i64]) -> i64 {
    let mut total: i64 = 0;
    let mut i: usize = 0;
    while i < s.len() {
        total = core::num::wrapping_add(total, s[i]);
        i = i + 1;
    }
    total
}

fn first(s: { s: &[i64] | s.len() > 0 }) -> i64 { s[0] }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(3);
    l.push(4);
    out.line(&sum(l.as_slice()))?; // expect: 7
    if !l.is_empty() {
        out.line(&first(l.as_slice()))?; // expect: 3
        out.line(&l[0])?;                // expect: 3
    }
    Result::Ok(())
}
```

`s[i]` is `*s.index(i)`, and `[T]::index` takes `i: { i: usize | i <
s.len() }`: the checker proves it from `i < s.len()`, the loop's
condition, so it can't fail, and nothing here says `with panic`. A
refinement can now be of any type, too: `first` takes a slice that has a
value, and `!l.is_empty()` says so of `l.as_slice()`, as `as_slice` says
its slice is as long as the list.

## A measure is a function of a value

A method is a measure if it's marked `#[measure]`, and then it must be
one: it takes only `&self`, has no type parameters of its own, gives an
integer or a `bool`, has no effects, and its body is a term of the logic,
of `self`'s fields, other measures, literals, `+`, `-`, `*` by a
constant, comparisons, `&&`, `||` and `!`. Or it's an intrinsic, whose
value the compiler provides: a slice's or a string's `len`, a `RawBuf`'s
`cap`.

```rust
mod window {
    /// The first `n` values of `data`.
    #[derive(Copy, Discard)]
    pub struct Window<'a> { data: &'a [u8], n: { n: usize | n <= data.len() } }

    impl<'a> Window<'a> {
        pub fn new(data: &'a [u8]) -> Window<'a> { Window { data, n: data.len() } }

        #[measure]
        pub fn len(&self) -> usize { self.n }

        /// `i < len()`, so `data` has it, as `n <= data.len()`.
        pub fn index(&self, i: { i: usize | i < self.len() }) -> &u8 { &self.data[i] }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let w = window::Window::new(b"abc");
    if w.len() > 1 {
        out.line(&w[1])?; // expect: 98
    }
    Result::Ok(())
}
```

That's what makes a measure mean the same in the logic as in the
program:

> **Lemma M1.** A measure is a total function of the value its `&self`
> points to.

What it reads, it reads through `&self`, and nothing in T changes
through a `&`: no interior mutability. It has no effects, so it can't
allocate, panic or read input, and the checker proves its arithmetic
can't overflow, as for any function that doesn't say `with panic`. Its
body has no loops, and measures defined by each other are an error (a
cycle would say nothing: `a() == !b()` and `b() == !a()`), so it ends.
So in the logic it's a function symbol, and the program's call of it
gives that function's value.

## Its body where it's declared, a symbol elsewhere

In its module, a measure is its body: `w.len()` is `w.n`, so `index`
above is proved from the field's refinement. Outside, a measure that
reads a field is only a symbol, and what's known of it is what's been
learned: from calling it, and from refinements, as `index`'s. That keeps
its module's abstraction (docs/06d-modules.md): code outside can't come to
depend on what the body is, so changing it breaks nothing there, as long
as the refinements still hold.

A measure whose body reads no field, only other measures, is its body
everywhere: `is_empty` is `self.len() == 0`, so `!l.is_empty()` tells
the checker `l.len() > 0`, which `l[0]` needs. Its body says no more than
its name, so seeing it hides nothing.

## Which value: objects

A measure is a function of a value, so the checker needs to know when two
values are the same. Each value that isn't an integer or a `bool` is an
*object*, an identity that never changes, as a variable of the logic is,
and `m(o)` is one variable, however many times it's asked. A copy is the
same object, so after `let t = s;`, `t.len()` is `s.len()`. A place, as
`s` or `self.data`, is an object until it may change: assigned, lent
`&mut`, or changed in a loop, where it becomes a new one, as an integer
place becomes a new variable (docs/08.1-field-refinements.md).

This is the theory of *equality with uninterpreted functions*, decided
here the simple way: two objects are equal only if they're the same one.
That's sound, and incomplete: two different slices with the same length
aren't known to have the same length.

A slice's measure, or a string's, is of the reference, not of what it
points to. A `&[T]` is a pointer and a length, and nothing written
through a `&mut [T]` can change its length, only its values:

```rust
fn bump(s: &mut [i64], i: { i: usize | i < s.len() }) with panic {
    s[i] += 1;
    s.swap(0, i);
    s[i] += 1;
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(10);
    l.push(20);
    let s = l.as_mut_slice();
    if s.len() == 2 {
        bump(s, 1);
        out.line(&s[0])?; // expect: 21
        out.line(&s[1])?; // expect: 11
    }
    Result::Ok(())
}
```

`s.swap(0, i)` is lent `&mut *s`, which may change every value of the
slice, but `s` is the same reference after, so `i < s.len()` still holds.
A reborrow, `&*s`, is the same object too.

## `x[i]`

`x[i]` is `*x.index(i)`, and where it's written to, as `x[i] = v`, `x[i]
+= v` or `&mut x[i]`, `*x.index_mut(i)`. The choice is the syntax's: the
place being assigned or borrowed `&mut`, and what it's in, through fields
and `*`s. They're methods found by name, so any type can be indexed, as
`List` is; a trait would do it better, once a trait's method can have a
refinement (`Index`, later).

What an assignment assigns is evaluated before the place, as in Rust, so
`s[0] = s[1]` reads `s[1]` before `index_mut` borrows `s`.

A method called on `x[i]` that takes `&mut self` would need `index_mut`,
which only the syntax around `x[i]` picks: write `(&mut x[i]).m()`.

## What's behind a `&mut`

A place lent `&mut` may change. Until now the checker forgot what it knew
of it after a call that couldn't keep the reference, and otherwise
treated it as changing anywhere in the body, which made `l[i] = v` forget
everything of `l` everywhere. Now it forgets it where it's lent: the
borrow checker says nothing reads the place while the borrow lives, so
whatever's written through the reference, the next read is after it
ends, and sees a value nothing's known of. This was step 8.1's
"deliberately missing" (docs/08.1-field-refinements.md); its mutation
testing had found it sound.

A refinement may name a measure of what's behind a `&mut` where it means
one value:

- a parameter's, which is checked at the call, and is about the
  arguments then: `index_mut(&mut self, i: { i: usize | i < self.len() })`;
- a result's, which is about the arguments *at the call* too, even if
  what's behind the `&mut` has changed by the time it returns;
- an immutable `let`'s, checked once.

Not a `mut` variable's or a field's, which are checked again later, when
what's behind the `&mut` may be something else.

```rust
use alloc::list::List;

/// How long the list was.
fn take_last(l: &mut List<i64>) -> { n: usize | n == l.len() } {
    let n = l.len();
    l.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(5);
    l.push(6);
    out.line(&take_last(&mut l))?; // expect: 2
    out.line(&l.len())?;           // expect: 1
    Result::Ok(())
}
```

That a result is about the call is what keeps the caller and the callee
in step: the caller knows what it passed, and the callee what it was
given, and neither knows what the other did after. It also means a
function can't say what it did to what's behind a `&mut`: after `l[2] =
7`, nothing's known of `l.len()`, as `index_mut` doesn't say it leaves it
as it was. A slice can: `let s = l.as_mut_slice();` is as long as `l` was,
and writing through `s` can't change that.

## Why it's sound

> Every fact the checker has is true of the values the program has at
> that point, the measures' values included; and every refinement it
> proved holds.

Step 8's and 8.1's induction, with objects. A fact about `m(o)` comes
from a call of `m` on `o`, whose value it is by M1; from a refinement
proved where it was bound, about the values then; or from `m`'s body, in
its module or when it reads no fields, which is what `m` is. An object
stands for one value: a place gets a new one whenever it may change, and
it may change only by assignment, which the checker sees, or through a
`&mut`, which it forgets where it's lent, as nothing reads the place again
until the borrow's over (the borrow checker), or in a closure, where the
place is a new object each time it's read. A refinement that names what's
behind a `&mut` is checked once, against the values its check sees, in
the callee and in the caller alike.

The interpreter checks it as it runs: each refinement, measures and all,
where it's checked, with what it says of names other than its binder
worked out when it's made, so a result's measures are of the arguments at
the call, as the checker's are; the intrinsics' refinements too, as what
they give is trusted; and `index` out of range stops it with "type
soundness violated".

## Design decisions worth noticing

- **A measure is marked.** It's a promise in the interface, that the
  method is a function of the value; inferring it would make a body
  change a breaking one.
- **Opaque outside its module, unless it reads no fields.** Changing a
  measure's body never breaks code outside, the same as changing a
  private field.
- **Identities, not a congruence closure.** Equal objects are the same
  object. A real decision procedure for uninterpreted functions would
  know more, and accepts more programs later without breaking any.
- **A slice's length is the reference's.** It's what makes a `&mut [T]`
  the way to write through a list and keep its length.
- **A result is about the call.** Postconditions on what a function does
  to its `&mut` arguments, `old(..)` and the value at return, are later:
  they need the caller to know when the borrow ends, as prophecies
  (RustHorn) or strong references (Flux) do.
- **`x[i]` by name, not a trait.** A trait's method can't carry a
  refinement yet: an impl's would have to ask no more than the trait's.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `List`'s `len <= cap()` as a field refinement, so `push` proves `len + 1` | an invariant kept while a field is lent `&mut`, as `buf` is to `grow` and `write` | — |
| what a `&mut self` method leaves as it was, `old(self.len())` | postconditions on what's behind a `&mut`, which need to know when the borrow ends | — |
| measures with arguments, `s.is_char_boundary(i)` | a function symbol of several arguments, and its congruence | — |
| refinements in type arguments, `Option<{ s: &str \| s.len() > 0 }>` | `split_at` relating its halves to what it split | — |
| `Index` as a trait | refinements in a trait's methods | — |
| `&s[1..3]` | a range, as an index | — |

## Exercises

1. Write `fn swap_ends(s: &mut [i64])` that swaps the first value and the
   last with `s[0]` and `s[n - 1]`, and no `with panic`. What does it need
   to know first?
2. Make `Window::len` read no field, by keeping `n` out of it. What can
   code outside the module now prove, and what can the module no longer
   change without breaking it?
3. Give `Window` a `shrink(&mut self)` that takes one off `n`. Why can't
   a caller then use `w[w.len() - 1]` after `w.shrink()` without asking
   `len()` again?
4. Why must a measure have no effects? Write one with `with panic` (by
   hand, in the checker) whose call panics, and a refinement that would
   then be "proved" of a value it doesn't hold of.
5. `List::index_mut` takes `&mut self`. Why can't its result's refinement
   say the list is as long after as before, and what would it take?
6. Make a result's refinement about what's behind a `&mut` mean its value
   at return in the callee, but at the call in the caller. Write the
   program that goes wrong.

## Reading

- Niki Vazou, Eric Seidel, Ranjit Jhala, Dimitrios Vytiniotis and Simon
  Peyton Jones, *Refinement Types for Haskell* (2014): measures, as
  Liquid Haskell has them.
- Greg Nelson and Derek Oppen, *Fast Decision Procedures Based on
  Congruence Closure* (1980): equality with uninterpreted functions done
  properly.
- Nico Lehmann, Adam Geller, Niki Vazou and Ranjit Jhala, *Flux: Liquid
  Types for Rust* (2023): refinements through `&mut`, by strong
  references.
- Yusuke Matsushita, Takeshi Tsukada and Naoki Kobayashi, *RustHorn* (2020),
  and *RustHornBelt* (2022): a `&mut`'s final value, as a prophecy.
- K. Rustan M. Leino, *Dafny* (2010): `old(..)` in postconditions.
