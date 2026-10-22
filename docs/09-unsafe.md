# Step 9: unsafe code and raw memory

Code: `alloc/raw.t` (`RawBuf`, with each obligation written down),
`src/check/effects.rs` (`Checker::unsafe_block`, which takes the effect
away, and the `unused_unsafe` lint), `src/safety.rs` (the
`missing_safety_comments` lint), `src/load.rs` and `src/check/modules.rs`
(the `alloc` package), and `src/interp.rs` (`Interp::raw_buf`, which
checks every obligation as the program runs).

```rust
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: both slots are uninitialised, and there are two.
    unsafe {
        b.write(0, 3);
        b.write(1, 4);
    }
    b.grow(8);
    // SAFETY: slots 0 and 1 hold the 3 and the 4, and are read out once
    // each, so every slot is uninitialised when the buffer is freed.
    unsafe {
        print(b.read(0) + b.read(1)); // expect: 7
        b.free();
    }
}
```

The standard library's `List` is to be a dense, resizable array, written
in T (see the README). For that T needs memory it can lay values out in,
which nothing so far gives it: a `Box` holds one value, and the type
system can't yet say "slots `0..len` hold values, and the rest don't".
Proving that is research (see *Deliberately missing*). What T has instead
is what Rust has: code the checker can't check, marked as such, kept
small, and trusted.

## Raw buffers

`RawBuf<T>` is a buffer on the heap of `cap()` *slots* for `T`s, each
*initialised* (holding a value) or not. `RawBuf::new(cap)` makes one with
every slot uninitialised, and `grow` gives it more. What uses a slot is
`unsafe`: `write` puts a value in one that's empty, `read` moves one out,
`slice` and `slice_mut` borrow a run of them as a slice (since step 10,
see docs/10-slices.md; until then it was `get` and `get_mut`, of one
slot), and `free` gives the memory back when they're all empty. Each has
an *obligation*, written in its `# Safety`, that the checker doesn't
check: the index is less than `cap()`, and the slot is initialised, or
isn't. That the index isn't negative is its type,
`{ i: int | i >= 0 }`, which the checker does prove (see
docs/08-refinement-types.md): the upper bound would need a refinement to
name `cap()`, which it can't yet. Nothing in the buffer remembers which slots hold
what. The code using it does, with `len` in a `List`.

A typed buffer rather than pointers into bytes keeps the obligations
local: each is about one slot of one buffer. There's no pointer
arithmetic, no casts, and no aliasing rules beyond the borrow checker's:
`slice` borrows the buffer as `&self` and `slice_mut` as `&mut self`, so
a slice of it can't outlive it, nor a `grow` that moves the slots.

```rust,error
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised, then holds the 5.
    let s = unsafe {
        b.write(0, 5);
        b.slice(0, 1)
    };
    b.grow(8);
    print(s.len());
    // SAFETY: slot 0 holds the 5; then every slot is uninitialised.
    unsafe {
        b.read(0);
        b.free();
    }
}
// error: cannot borrow `b` as mutable while `b` is borrowed
```

A `RawBuf` isn't `Discard` (docs/06a.1-linear-types.md), so it must be
used up, by `free`: its memory is never leaked, nor freed twice, and as
`free` needs every slot empty, no value in it is lost either, linear ones
included. Nor is it `Copy`. It's an *intrinsic* type: only its own methods
make one or look inside, so there's no `RawBuf {}` literal or pattern, and
its type parameter is covariant, as `Box<T>`'s is, though it has no fields
to show that. `cap()` and `grow` are safe: `grow` panics if it would
shrink the buffer, which is cheap to check, and so isn't an obligation.

It's the only thing in the new `alloc` library: a package of its own, as
`core` is (docs/06e-core.md), which uses `core`, and which nothing is in
the prelude of. A program says `use alloc::raw::RawBuf;`. `core` still
needs no allocation.

## `unsafe` is an effect

An unsafe operation's obligation is its caller's to keep, and `unsafe` is
how the caller says so. It's an effect, as `panic` and `alloc` are
(docs/07-effects.md): `RawBuf::read` is `with unsafe`, and so is anything
that calls it, unless the call is in an `unsafe` block. A block *keeps*
the obligations of the operations in it, so it doesn't have the effect:

```rust
use alloc::raw::RawBuf;

/// The caller must know slot 0 holds a value.
fn first(b: &RawBuf<int>) -> Option<&int> with unsafe { b.slice(0, 1).first() }

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    // SAFETY: slot 0 is uninitialised; then it holds the 7, which is read
    // out before the buffer is freed.
    unsafe {
        b.write(0, 7);
        print(*first(&b).unwrap()); // expect: 7
        b.read(0);
        b.free();
    }
}
```

A function with an obligation of its own says `with unsafe`, and passes it
on to its callers, as `first` does. One that keeps all its obligations in
blocks doesn't, and is as safe to call as any other. `main` can't say
`with unsafe`: nothing calls it to keep its obligations.

Everything the effects already did, `unsafe` gets for free. A function
type says it: an unsafe `fn` won't do as a `fn(int) -> int`. An `Fn`
bound says it: `F: Fn() -> int` doesn't hold of an unsafe closure. A
trait's method says what its impls may do. And effect parameters carry it
through generic code: `apply(f)` of an unsafe `f` is unsafe.

What a block takes away is only the `unsafe` of the operations written in
it, not what an effect parameter turns out to be:

```rust,error
use alloc::raw::RawBuf;

fn call<e, F: FnOnce() -> int with e>(f: F) -> int with e {
    // SAFETY: nothing here is unsafe but `f`, whose obligations aren't ours.
    unsafe { f() }
}

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(1);
    print(call(|| b.read(0)));
    // SAFETY: there's nothing in the slot to leave behind.
    unsafe { b.free() };
}
// warning: this `unsafe` block has nothing unsafe in it
// error: `call` is unsafe, but it isn't in an `unsafe` block
```

`call` can't know what obligations `f` has, so it can't keep them, and
its block doesn't: `e` stays `call`'s effect, and `main`, which passes the
unsafe closure, must keep them. Nor does a block keep what a closure made
in it does when it's called later: a closure's effects are its body's,
and its body's `unsafe` is kept, or not, where it's called. A block inside
the closure's body keeps it there.

Every `unsafe` block should say why its obligations are kept, in a
`// SAFETY:` comment on the lines just above it: the
`missing_safety_comments` lint warns of one that doesn't. And
`unused_unsafe` warns of a block with nothing unsafe in it, which only
hides where the unsafe operations aren't.

## Broken obligations

The checker trusts every block. The interpreter doesn't: it checks every
obligation as the program runs, as Miri does for Rust, and stops at the
first that's broken, blaming the innermost block running:

```rust,error
use alloc::raw::RawBuf;

fn main() with {alloc, panic} {
    let mut b: RawBuf<int> = RawBuf::new(2);
    // SAFETY: wrong: slot 1 was never written.
    unsafe {
        b.write(0, 1);
        print(b.read(1));
        b.read(0);
        b.free();
    }
}
// error: unsafe code broke its obligation: `RawBuf::read` of slot 1, which is uninitialised
```

That's an error of the program's, not "type soundness violated", which
stays the checker's: a bug in a block is a bug in that block, which the
checker never promised anything about. The interpreter keeps a buffer's
slots on its heap, as it does a box's value, so a reference into one is
into that, and `grow` moves them somewhere new: a reference that outlived
it would dangle, and the borrow checker's rules are checked as they were.
A `RawBuf` left at the end is a leak.

## Why it's sound

> A program whose `unsafe` blocks keep their operations' obligations
> doesn't go wrong.

The trusted part is small: `RawBuf`'s methods, and each `unsafe` block.
The rest is the effect lemma of step 7, with `unsafe` as the effect: a
function without `with unsafe` never reaches an unsafe operation but
inside a block, as every site's effects are among its function's, or its
closure's, and the only thing that takes `unsafe` away is a block, which
takes away only that of the operations in it. So every unsafe operation
that runs is either in a block, which kept its obligation by the
assumption, or in a function that says `with unsafe`, called from a block
or from one that says it too. `main` doesn't, so the chain ends in a
block.

The obligations make each operation safe on its own: an index in range,
`>= 0` by its type and `< cap()` by the obligation, is memory the buffer
has; a slot written only when it's empty loses no
value; read, `slice` and `slice_mut` only of slots that hold one. What they
don't cover, the types do. A buffer is linear, so it's freed once, and
then only when it's empty, so nothing in it is lost, and nothing is left
pointing into it: `slice` and `slice_mut` borrow it, and `free` takes it by
value, as `grow` takes it by `&mut`. Its type parameter is covariant,
which is sound for what owns its `T`s, and its regions are its type's
arguments', so a buffer of `&'a int` can't outlive `'a`.

## Design decisions worth noticing

- **A typed buffer, not raw pointers.** Each obligation is about one slot
  of one buffer, and the borrow checker still checks every reference.
  Rust's `*mut T`, pointer arithmetic and `alloc`/`dealloc` would need an
  aliasing model, which T doesn't have.
- **`unsafe` is an effect**, not a marker on functions: function types,
  bounds, traits and generic code all say whether something is unsafe the
  way they say whether it panics, with nothing new to check.
- **A block keeps only what's written in it.** Not what a type parameter
  stands for, and not what a closure does later: those obligations are
  kept where what has them is known.
- **The interpreter checks every obligation**, with an error of its own,
  so the tests of `alloc` find the bugs in its blocks.
- **No destructors yet.** A buffer is used up by `free`, so a `List` will
  be linear too: used up by a method of its own, rather than freed when
  it's dropped, which would be a call nobody wrote (see
  docs/06a.1-linear-types.md).

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `alloc::list::List<T>`, the dense resizable array, and iterating it | built on `RawBuf`, with its own obligations to get right | 9.1 ✅ |
| destructors, `Discard` with a `fn discard(self)` | a drop would be a call nobody wrote | — |
| raw pointers, byte buffers, casts | need an aliasing model, and layouts | — |
| failing allocation, `RawBuf::try_new` | needs `Result` from an intrinsic | — |
| an index's upper bound as a refinement, `i < self.cap()` | measures: refinements that name what a method returns | — |
| checking initialisation, as ATS's views do | linear capabilities for each slot: research | — |

## Exercises

1. Write a `Stack` of `int`s on a `RawBuf<int>`, with `push`, `pop` and
   `free`, keeping `len <= cap()`. Which of its methods need `unsafe`
   blocks, and what does each `// SAFETY:` say?
2. Why can't `free` be safe for a `RawBuf<T>` with `T: Discard`, when
   dropping each value in it would do nothing?
3. Make `RawBuf` invariant in `T` instead of covariant. Which programs
   stop checking? Then make it bivariant, and write the program that goes
   wrong.
4. Why would an `unsafe` block that took away an effect parameter's
   `unsafe` be unsound? Write the generic function and the caller.
5. `grow` panics rather than having an obligation, and an index's lower
   bound is a refinement. Find another of `RawBuf`'s obligations that
   could be a check instead, and say what it would cost a `List`.

## Reading

- Jung, Jourdan, Krebbers and Dreyer, *RustBelt: Securing the Foundations
  of the Rust Programming Language* (2018): what it means for unsafe code
  behind a safe interface to be sound.
- Jung et al., *Stacked Borrows* (2020), and the Miri interpreter: checking
  unsafe code's obligations as it runs.
- Xi, *Applied Type System* (2004), and ATS's views: proving
  initialisation with linear types, rather than trusting it.
- Leijen, *Koka: Programming with Row Polymorphic Effect Types* (2014):
  the effects `unsafe` is one of.
