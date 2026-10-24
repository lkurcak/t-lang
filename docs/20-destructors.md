# Step 20: destructors

Code: `core/mem.t` (`Discard::discard`), `alloc/list.t` and
`alloc/string.t` (a list's `discard`, a string's derive),
`src/check/traits/collect.rs` and `impls.rs` (where a `discard` may be
written), `src/borrowck/moves.rs` (`drop_uses`, `leftover`,
`abstraction`), `src/borrowck/regions.rs` (`liveness`), `src/mono.rs`
(`Target::Discard`, the copies of each `discard`) and `src/interp.rs`
(`Interp::drop`).

Since step 15 a `?` can return early, and since step 19 every write
gives a `Result`. But a value that isn't `Discard` can't be dropped on the
way out, and a `String` and a `List` weren't: so a function that held one
couldn't use `?` until it had freed it (docs/19-std-io.md). Now a type can
say how it's thrown away, and a string and a list do:

```rust
use alloc::list::List;
use alloc::string::String;

fn shout(out: &mut std::io::Stdout, words: &List<String>) -> Result<(), std::io::Error> with alloc {
    let mut s = String::new();
    for w in words.iter() {
        s.push_str(w.as_str());
        s.push('!');
    }
    out.line(&s.as_str())?; // expect: hey!you!
    out.line(&s.len())      // expect: 8
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut words = List::new();
    words.push(String::from("hey"));
    words.push(String::from("you"));
    shout(&mut out, &words)
}
```

Every `String` and the `List` here is freed: `s` at the end of `shout`,
or on the way out of its `?`, and `words`, with each string in it, at the
end of `main`. No `free` is written, and none is missing: the
interpreter checks every buffer is given back.

## `Discard` is weakening, and now it says how

A type system without *weakening*, the rule that lets a variable go
unused, is linear: every value must be used up (docs/06a.1-linear-types.md).
`Discard` gave weakening back, type by type, for those whose values may
just be forgotten. In linear logic, a type has weakening when there's a
map from it to the unit, `A ⊸ 1`: a way to use a value up that gives
nothing back. `Discard` now has that map as its method:

```text
pub trait Discard {
    fn discard(self);
}
```

Wherever a value is dropped, at the end of its scope, when its place is
given a new value, or what a pattern leaves of it, its type's `discard`
is what's called. For the built-in types and a derived `Discard` it drops
the parts, as dropping did before: nothing, for an `i64`; freeing the
box, and dropping what's in it, for a `Box<T>`; each field, in order, for
a struct. A struct or enum may instead say what to do, with an impl:

```text
impl<T: Discard> Discard for List<T> {
    fn discard(self) { self.free_with(|_x| {}) }
}
```

That's what `List::free` was, and `free` is gone: dropping a list is
calling it. `String` derives `Discard`, as its one field is a list. A
list of values that aren't `Discard` isn't either, and is still used up by
`free_with`. `x.discard()`, called by name, drops `x` now, rather than at
the end of its scope, for any type that's `Discard`.

A `discard` of a program's own may do anything a total function may:

```rust
struct Noisy<'a> { out: &'a mut std::io::Stdout, n: i64 }

impl<'a> Discard for Noisy<'a> {
    fn discard(self) {
        let _ = self.out.line(&self.n);
    }
}

fn main(os: std::Os) {
    let mut out = os.stdout;
    let a = Noisy { out: &mut out, n: 1 };
    {
        let _b = Noisy { out: &mut *a.out, n: 2 };
    } // expect: 2

    a.discard(); // expect: 1
}
```

Variables are dropped the last declared first, at the end of their
block (or where `x.discard()` says), and a struct's fields in the order they're declared, after its
own `discard`, if it has one, has run.

## Implicit code must be total

> **Lemma D2 (what's inserted can't fail).** `discard` has no effects:
> the trait's method says none, and an impl's method can't say more than
> its trait's (docs/07-effects.md). It gives back `()`. So calling it
> where a value is dropped, on every path, the early one of every `?`
> too, adds no effect to any function, and nothing to handle.

```rust,error
struct Loud { n: i64 }

impl Discard for Loud {
    fn discard(self) with panic {
        if self.n > 0 { core::panic::panic("loud") }
    }
}

fn main() {}
// error: `discard` may panic, but `Discard`'s `discard` can't, so its impls can't either
```

That's the line between what the compiler may call for you and what you
must call. `Copy` is contraction, `A ⊸ A ⊗ A`, done implicitly because a
copy is just the bits; a `Clone` that allocates would be called by name,
as it needs `alloc`. Freeing is total, so it can be implicit. Closing a
file can fail, so a file can't be `Discard`: it will be linear, closed by
a `close` that gives a `Result`, which the program must look at.

## A drop may read what's borrowed

A `discard` of a type's own may read through the references in its
value, as `Noisy`'s writes through its `&mut`. So dropping one is a use
of it, as a call would be:

```rust,error
struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) {
    let mut out = os.stdout;
    let _p;
    {
        let x = 5;
        _p = Peek { r: &x, out: &mut out };
    }
}
// error: `x` doesn't live long enough
```

> **Lemma D1 (a drop is a call).** Every point MIR drops a value, a
> `Dead`, an `Assign` over what was there, or a `Consume`'s leftovers, is
> where the interpreter drops it, and where a compiled program would call
> `discard`. If the value may still be there (it isn't definitely moved
> out), and its type may reach a `discard` of its own, the local is
> *live* at that point (`moves::drop_uses`), so every region in its type
> holds the point, and every loan it holds is still alive. Then what
> `discard` reads is valid there, by what a use is valid by.

Which types may reach one: a struct or enum with one; one that holds, or
whose box holds, such a value; a closure that captured one; and a type
parameter or a projection, which might be one. A value that only borrows
is dropped by freeing its memory, which reads nothing, so `let r;
{ let x = 1; r = &x; }` is still fine. An opaque type is what it stands
for, which is known.

It's conservative. A list's `discard` doesn't read what its values
borrow, but nothing says so:

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut l = List::new();
    let x: i64 = 5;
    l.push(&x);
}
// error: `x` doesn't live long enough
```

`x` is declared after `l`, so it goes first; declaring it first is the
fix. Rust's `#[may_dangle]` is an unsafe promise that a `discard`
doesn't read its parameters' borrows, which would let this through.

## Its own `self`

In a type's own `discard`, `self` isn't dropped by calling that
`discard` again: what's left of it at the end is dropped part by part, as
a derived one would drop it.

> **Lemma D3 (no self-recursion).** In `X::discard`, `self`'s drop at the
> end is its fields' drops, each by its own type's `discard`, in the
> checker (`moves::leftover`) and the interpreter (`Interp::end_scope`)
> alike. So a field that isn't `Discard` must be used up there.

```rust,error
use alloc::raw::RawBuf;

struct Buf { raw: RawBuf<i64> }

impl Discard for Buf {
    fn discard(self) {}
}

fn main() with alloc {
    let _b = Buf { raw: RawBuf::new(1) };
}
// error: `self.raw` is never used up
```

Other values of `X` in its `discard`, like the rest of a linked list, are
dropped by `X`'s `discard`, which ends as the value does.

## Only its module may skip it

A `discard` of its own is a promise: whatever the type's module means by
throwing a value away is done. Taking a value apart, moving out of it or
matching it by value, ends it another way, without calling `discard`,
which is how the module ends one on purpose, as `close` here does:

```rust
mod file {
    pub struct File<'a> { pub out: &'a mut std::io::Stdout, pub fd: i64 }

    impl<'a> Discard for File<'a> {
        fn discard(self) {
            let _ = self.out.line(&"discarded");
        }
    }

    impl<'a> File<'a> {
        pub fn close(self) -> i64 {
            let File { out: _, fd } = self;
            fd
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = (file::File { out: &mut out, fd: 7 }).close();
    {
        let _f = file::File { out: &mut out, fd: 8 };
    } // expect: discarded

    out.line(&fd) // expect: 7
}
```

> **Lemma D4 (only its module skips it).** A by-value pattern on a value
> whose type has a `discard` of its own uses it up, as one on a linear
> value does, even if it moves nothing out. Taking such a value apart,
> by a pattern or by moving a field out, is allowed only in the type's
> module (`moves::abstraction`), and its `discard` must be written there
> too. So outside it, every value of the type ends by its `discard`.

```rust,error
mod file {
    pub struct File { pub fd: i64 }

    impl Discard for File {
        fn discard(self) {}
    }
}

fn main() {
    let f = file::File { fd: 1 };
    let file::File { fd: _ } = f;
}
// error: can't take `File` apart here: it has a `discard` of its own, which only `file` may skip
```

This extends lemma M2 of docs/06d-modules.md, which said the same of a
type with private fields that isn't `Discard`. Reading a field, `f.fd`,
or matching through a reference, `&f`, takes nothing apart.

## What an impl may say

> **Lemma D5 (`Copy` has none).** A `Copy` type can't have a `discard` of
> its own: its copies are made and dropped where nothing says so, so
> dropping one must be nothing.

An impl of `Discard` may need only that the type's parameters are
`Discard`, as a derived one does, so that proving a type `Discard` needs
only more of the same, never another trait: lemma D1 of
docs/17-retrospective-1.md still holds. A derived and a written impl are
two impls, which overlap. And an impl with no `discard` says no more than
the derive, which is the fix.

```rust,error
#[derive(Copy)]
struct Id { n: i64 }

impl Discard for Id {
    fn discard(self) {}
}

fn main() {}
// error: `Id` is `Copy`, so it can't have a `discard` of its own
```

> **Lemma D6 (one meaning).** Generic code drops a `T: Discard` by `T`'s
> `discard`, as if passed in a dictionary, and the copy monomorphisation
> makes at `T = List<i64>` calls `List<i64>`'s. Monomorphisation makes a
> copy of each type's own `discard` with the type, and the interpreter
> calls it wherever it drops a value of the type that hasn't been taken
> apart, as the checker has it. Its leak check, that nothing that isn't
> `Discard` is ever dropped and every buffer is freed, still runs on
> every test.

## Design decisions worth noticing

- **One concept, not two.** Rust has a `Drop` trait and drop glue, and
  `Copy` and `Drop` exclude each other by a rule of their own. Here
  `Discard` is weakening and its method is how: derived, it drops the
  parts; written, it's the type's.
- **Not a second trait for "drops trivially".** Generic code only asks
  whether a value may be dropped, never whether that runs code, and a
  bound that said it doesn't would make adding a `discard` to a type a
  breaking change: Rust's `T: Drop` bounds are a lint for that reason. The
  checker knows which types reach a `discard` of their own, for the drop
  check and for `Copy` (D5), without a trait. If one's ever needed, a
  subtrait of `Discard` can be added without breaking anything. The split
  that may matter later is another one: what may be skipped, as freeing
  memory may, and what must run, as a lock's guard, once something like
  `forget` or a reference-counted pointer could skip one.
- **Still `Discard`, not `Drop`.** The trait is mostly in bounds, `T:
  Discard`, "may be thrown away", which `Drop` would get backwards for a
  reader who knows Rust, where everything may be dropped and `Drop` means
  having a destructor. The event is a *drop*; the trait and its method are
  what allow it, as a value of a `Copy` type is *copied*.
- **By value, not `&mut self`.** Rust's `drop(&mut self)` leaves the
  fields to be dropped after, and can't move them out. `discard(self)` may
  take `self` apart, and give its parts to whoever should have them.
- **Implicit only if total.** It's what lets drops be on every path, and
  keeps a fallible end, as a file's, explicit.
- **Conservative drop check.** Any drop that may reach a `discard` of its
  own uses the whole value. Relaxing it later, with a promise like
  `#[may_dangle]`, accepts more programs and breaks none.
- **The cost of not `free`ing.** A list is freed at the end of its scope,
  not where it was last used, unless `x.discard()` says so: memory lives
  a little longer, and a drop is a call that's not written down, as in
  Rust and C++.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `#[may_dangle]` | an unsafe promise, for a list holding borrows of what's declared after it | — |
| `try` blocks, `defer` | a value that's linear on purpose still can't leave on `?` without a bracket | — |
| `core::mem::forget` | a leak is safe in Rust, but would break D4's promise here: only a module may skip its own `discard` | — |
| files, closed by `close` | linear values in `std`, with errors | — |

## Exercises

1. Write a `Guard` that holds a `&mut List<i64>` and pops its last value
   when it's dropped. Can `discard` push instead? Why not?
2. Make a struct `Node { next: Option<Box<Node>>, n: i64 }` with a
   `discard` of its own that writes `n`. In what order are the nodes of a
   list of three dropped? How deep does the interpreter's stack get for a
   long one, and how would a loop in `discard` avoid it?
3. Write `forget<T: Discard>(x: T)` in T. Can it skip a `discard` of its
   own? Why does D4 matter for a type whose module counts on its `discard`
   running, as a lock's guard would?
4. Let a `discard` say `with panic`. Which function would then have to
   say it too, in the first program of this page, and why is that a
   reason to forbid it rather than to infer it?
5. Take `drop_uses` out (make it give nothing). Which tests fail, and how
   does the interpreter find each?
6. Why is it right to look through an opaque type, `impl FnMut()`, to the
   closure it stands for, in the drop check, but not in deciding whether
   it's `Discard`? What does a function then promise its callers about
   the closure it returns?

## Reading

- Jean-Yves Girard, *Linear logic* (1987): weakening and contraction as
  structural rules, and the exponential `!A` that gives them back.
- Philip Wadler, *Linear types can change the world!* (1990).
- *The Rustonomicon*, "Destructors", "Drop Check" and "Leaking": drop
  glue, `#[may_dangle]`, and why `mem::forget` is safe in Rust.
- Bjarne Stroustrup, *The Design and Evolution of C++* (1994), on RAII:
  resource acquisition is initialisation.
- Aria Beingessner, *Pre-Pooping Your Pants With Rust* (2015): the
  leakpocalypse, and what a guard may count on.
