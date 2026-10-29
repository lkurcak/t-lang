# Step 23: arenas

Code: `alloc/arena.t` (`Arena<T>`, an intrinsic type), `Interp::arena` in
`src/interp.rs`, `reaches_own_discard` in `src/borrowck/moves.rs` (what
dropping an intrinsic type reaches), and the variances of intrinsic types
in `src/borrowck/regions.rs`.

Until now, each value on the heap was a box of its own, freed on its own,
or one of a `List`'s, which can't be pointed into while it may grow. An
*arena* is memory for values that all live as long as it does, and go
together, when it does. A reference into it lives as long as the borrow
of the arena, so values in it can point to each other: a list's nodes, a
tree's, or a graph's, with no box for each and no freeing one by one.
That's *region-based memory*, the regions being the lifetimes the borrow
checker already has.

```rust
use alloc::arena::Arena;

#[derive(Discard)]
struct Node<'a> { v: i64, next: Option<&'a Node<'a>> }

fn sum(n: &Node) -> i64 with panic {
    let mut total = 0;
    let mut at = Option::Some(n);
    loop {
        match at {
            Option::Some(m) => {
                total = total + m.v;
                at = m.next;
            },
            Option::None => return total,
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = Arena::new();
    let mut head: &Node = a.alloc(Node { v: 1, next: Option::None });
    for i in 2..5 {
        head = a.alloc(Node { v: i, next: Option::Some(head) });
    }
    out.line(&sum(head))?; // expect: 10
    let x = a.alloc(Node { v: 0, next: Option::None });
    let y = a.alloc(Node { v: 0, next: Option::None });
    x.v = 5;
    y.v = 6;
    out.line(&(x.v + y.v))?; // expect: 11
    out.line(&a.len())?;     // expect: 6
    Result::Ok(())
}
```

`alloc(&self, v)` moves `v` into the arena and gives back a `&mut` to it
there: from a `&`, so many at once, as `x` and `y` are. Nothing else
reaches what's in an arena: there's no `get`, and no iterating over it.
When it's dropped, so is each value in it, first put in first, and its
memory is given back all at once.

## Why it's sound

> **Lemma R1 (fresh).** Each `alloc` returns a reference to a value no
> other reference has been to, and no method reaches a value already in
> the arena. So the `&mut`s it hands out never alias.

> **Lemma R2 (stable).** No value in an arena moves or is freed before the
> arena is dropped. Each reference `alloc` returns borrows the arena, for
> as long as it lives, so the arena can't be dropped or moved meanwhile.

R1 is by the intrinsics: `alloc` puts each value somewhere new, `new` and
`len` touch none. R2 is the signature, `fn alloc(&self, v: T) -> &mut T`,
whose result has the lifetime of the `&self`: the borrow checker's own
argument does the rest.

```rust,error
use alloc::arena::Arena;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let r;
    {
        let a = Arena::new();
        r = a.alloc(5i64);
    }
    out.line(r)?;
    Result::Ok(())
}
// error: `a` doesn't live long enough
```

> **Lemma R3 (invariant).** `Arena<T>` is invariant in `T`.

A `Box<T>` or a `RawBuf<T>` is covariant in `T`: a `Box<&'static str>` is
a `Box<&'a str>`, as what's in it lives long enough. But an arena takes
values in through a `&`, as a `Cell` would, and a `&'b Arena<&'static
str>` seen as a `&'b Arena<&'a str>` could have a `&'a str` put in, which
the arena, of `&'static str`s, would hold past `'a`. An intrinsic type has
no fields for its variance to be worked out from, so it's fixed: `Arena`'s
is the only one that's invariant.

```rust,error
use alloc::arena::Arena;

fn shorten<'a>(a: &'a Arena<&'static str>) -> &'a Arena<&'a str> { a }

fn main() {}
// error: lifetime may not live long enough
```

> **Lemma R4 (dropping).** Dropping an arena drops each value in it, so
> if one may have a `discard` of its own, dropping the arena is a use of
> everything its type borrows.

That's step 20's drop check (lemma D1 of docs/20-destructors.md), which
until now looked for a `discard` among a type's fields; an intrinsic type
has none, so it now looks in its type arguments. When nothing in `T` has a
`discard` of its own, dropping the values follows no reference, so what
they point to may die first, the arena's own values among them: that's
how `Node`s point to each other above. When something does, it must not:

```rust,error
use alloc::arena::Arena;

struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) with alloc {
    let mut out = os.stdout;
    let a = Arena::new();
    let x = 5;
    let _ = a.alloc(Peek { r: &x, out: &mut out });
}
// error: `x` doesn't live long enough
```

`x` is dropped before `a`, which was declared first, and dropping `a`
would call `Peek`'s `discard`, which reads `x`. Rust needs an unsafe
promise, `#[may_dangle]`, to accept the first program and reject this
one; here both follow from what a `discard` can reach.

> **Lemma R5 (no leaks).** `Arena<T>` is `Discard` exactly when `T` is, so
> an arena of values that must be used up is one too.

The interpreter checks all five as it runs: each value put in an arena
has a heap slot of its own, freed when the arena is, so a reference that
outlived it would be to a freed slot; each `alloc` checks that every
function running says `with alloc`; and the leak check sees the arena's
values freed.

## Allocators: where this is going

The design pass before this step asked what memory should be, beyond the
one heap. What it decided is a direction, recorded here:

- **The heap stays ambient.** Any function may allocate from the heap, if
  it says `with alloc`. That isn't the ambient authority `print` was
  (docs/19-std-io.md): output is the world's, and nothing recorded it,
  but allocating is recorded by its effect. Code with no heap, a kernel's,
  is code with no `alloc`, which uses memory it's handed.
- **Allocators will be values.** A `trait Allocator` with an associated
  effect, as `Write` has: the heap's would be `alloc`, a fixed buffer's
  none, its allocations fallible instead. A container would hold its
  allocator, `List<T, A>`, so that its `discard` can still give memory
  back. Passing one to each call, as Zig does, would make `List` linear
  again.
- **Without breaking `List<T>`.** With default type parameters, `struct
  List<T, A: Allocator = Global>`, every `List<T>` written so far stays
  what it is. So allocator-generic containers wait for a backend whose
  memory use can be measured.
- **`alloc` says two things.** That a function allocates, and that it may
  abort when memory runs out (docs/16-out-of-memory.md). Fallible
  allocation, as `try_push`, allocates and doesn't abort; which of the two
  `alloc` keeps is decided with it.

## Design decisions worth noticing

- **Typed, and intrinsic.** Handing out `&mut`s from a `&` needs either a
  primitive the checker trusts or interior mutability, which T doesn't
  have. An `Arena<T>` of one type can drop its values, so it can hold any
  `T`; a bump arena of values of many types couldn't know how, so it
  would have to hold only `Copy` ones.
- **Nothing reaches its values but what `alloc` gave.** No `get`, no
  `iter`: either would be a second way to a value already handed out.
- **Its values are dropped.** An arena that skipped their `discard`s
  would leak a `String`'s memory, or worse, a linear value.
- **`alloc`, not `push`.** It's what typed arenas are called elsewhere,
  and it says the value gets memory of its own.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| allocator-generic containers, `List<T, A>`, `Box<T, A>` | default type parameters first, and a backend to measure them by | — |
| a bump arena of values of any `Copy` type | memory of no one type, so an untyped primitive | — |
| `into_iter`, `free_with` for an arena of linear values | moving every value out, which no reference may still point to | — |
| `Cell`, `RefCell` | interior mutability, its own soundness argument | — |
| cycles, `a.next = b; b.next = a` | setting a field through a `&`, interior mutability again | — |

## Exercises

1. Why must `alloc` take `&self`, not `&mut self`? Write the list of
   nodes above with a `&mut self` `alloc`, and see where it fails.
2. Add `fn iter(&self) -> Iter<T>` over an arena's values. Which lemma
   breaks, and what program shows it?
3. Make `Arena` covariant (in `src/borrowck/regions.rs`) and write a
   program that reads a dangling reference. Does it need a `get`?
4. A bump arena holds values of many types, `fn alloc<T: Copy>(&self, v:
   T) -> &mut T`. Why is `Copy` enough for dropping nothing to be sound,
   when `Discard` isn't?
5. Write `List<T, A: Allocator>` by hand: what does `push` need of `A`,
   and what does `discard`? Where does `A`'s effect show?

## Reading

- Mads Tofte and Jean-Pierre Talpin, *Region-Based Memory Management*
  (1997): memory in regions whose lifetimes are inferred, freed a region
  at a time.
- Dan Grossman et al., *Region-Based Memory Management in Cyclone*
  (2002): regions in a C-like language, checked by the types, as
  lifetimes are here.
- Simon Sapin's `typed-arena` and Nick Fitzgerald's `bumpalo` crates for
  Rust: a typed arena and a bump arena, and their `unsafe` arguments.
- The Rust RFC 1327, *Dropck parametricity* (2015), and its
  `#[may_dangle]`: when dropping a value may let what it points to die
  first.
- Andrew Kelley, the Zig standard library's `Allocator`: allocators as
  values handed to what allocates.
