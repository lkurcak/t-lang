# Step 46: one cell primitive, and the intrinsics by name

Code: `core::cell::Unsafe` in `core/cell.t`, and `Cell`, `Atomic`,
`Mutex`, `Volatile` and `Arena` over it (`core/cell.t`, `core/sync.t`,
`core/volatile.t`, `alloc/arena.t`); `check::UNSAFE`, `types::frozen`,
the variances in `src/borrowck/regions.rs`, `Checker::unsafe_impl` in
`src/check/traits/collect.rs`; `src/intrinsic.rs`, and `Interp::intrinsic`
and `Gen::intrinsic_body`, which match on it.

Retrospective 4 found two things the compiler knew by name that it
shouldn't have to (J1, J6 of the retrospective):

- Four types changed through a `&`: `Cell`, `Atomic`, `Volatile` and
  `Arena`. Each was listed by symbol, in up to five places: what's
  invariant, what's frozen, what's never `Sync`, what the backends lay
  out. Each had a lemma of its own (K2, Y7, V, R3). A library couldn't
  write a fifth, a cell set once (`OnceCell`) or a lock of its own,
  without asking for a compiler change.
- The intrinsics were strings: a list of 58 symbols in the checker, then
  matched again, by prefixes of their copies' names, in the interpreter
  and in the C backend. One that a backend missed was a crash when a
  program first called it.

This step makes the first a library matter and the second a compile
error of the compiler's.

## `Unsafe<T>`

`core::cell::Unsafe<T>` is the one type whose contents may change while
a `&` to it lives:

```rust
use core::cell::Unsafe;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let c = Unsafe::new(1u32);
    // SAFETY: no other reference to what's in `c` lives meanwhile.
    unsafe {
        *c.as_mut() = 2;
    }
    let mut out = os.stdout;
    // SAFETY: no `as_mut` reference to it lives while this one does.
    let r = unsafe { c.as_ref() };
    out.line(r)?; // expect: 2
    Ok(())
}
```

Its API is the smallest that lets a library build cells on it:

| | |
|---|---|
| `new(value)`, `into_inner(self)` | safe, as for any value |
| `get_mut(&mut self) -> &mut T` | safe: the `&mut` makes it the only way there |
| `as_mut(&self) -> &mut T with unsafe` | while the reference lives, nothing else reaches what's in it |
| `as_ref(&self) -> &T with unsafe` | while the reference lives, no `as_mut` reference to it does |

Like `Cell` before it (docs/37-cell.md), it's a struct with one private
field, so layout, drop glue, `Discard`, the interpreter's values and the
C struct are what any struct's are. Only `as_mut` and `as_ref` are
intrinsics, each a reference to that field. It derives `Discard` and
`Send` where `T` does, and never `Sync` or `Copy`.

The checker knows one thing by its name, `check::UNSAFE`: that it's
invariant in `T` and not frozen. Everything else follows from fields.

## Everything else is a struct over it

```rust
use core::cell::{Cell, Unsafe};

// A cell set once, then lent by `&` for as long as it lives: a library's
// own, which the compiler knows nothing about.
#[derive(Discard where T: Discard)]
struct Once<T> { set: Cell<bool>, value: Unsafe<Option<T>> }

impl<T: Discard> Once<T> {
    fn new() -> Once<T> { Once { set: Cell::new(false), value: Unsafe::new(None) } }

    fn get(&self) -> Option<&T> {
        if !self.set.get() {
            return None;
        }
        // SAFETY: once set, it's never lent as `&mut` again.
        let value = unsafe { self.value.as_ref() };
        value.as_ref()
    }

    fn set(&self, v: T) -> bool {
        if self.set.get() {
            return false;
        }
        // SAFETY: it isn't set, so no `&` from `get` lives, and this `&mut`
        // lives only for the assignment, which drops a `None`.
        unsafe {
            *self.value.as_mut() = Some(v);
        }
        self.set.set(true);
        true
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let once: Once<i64> = Once::new();
    let first = once.set(7);
    let second = once.set(8);
    let mut out = os.stdout;
    out.line(&first)?;  // expect: true
    out.line(&second)?; // expect: false
    match once.get() {
        Some(v) => out.line(v)?, // expect: 7
        None => {},
    }
    Ok(())
}
```

The library's own cells are written the same way:

- `Cell<T> { value: Unsafe<T> }`. `get` copies out through `as_ref`,
  `replace` is `mem::replace` through `as_mut`: in both the reference
  lives for one step, which runs no code of the program's. `Cell` lends
  nothing out any more: its `as_mut`, which step 41 added for `Mutex`, is
  gone, as `Unsafe::as_mut` is what a lock needs.
- `Atomic<T> { value: Unsafe<T> }`, with
  `unsafe impl<T: Atom + Send> Sync`: each change is one atomic step,
  which is the promise. Its methods stay intrinsics.
- `Mutex<T> { locked: Atomic<bool>, value: Unsafe<T> }`, as before but
  for the `Unsafe`.
- `Volatile<T> { value: Unsafe<T> }`, neither `Send` nor `Sync`, as it
  says neither.
- `Arena<T> { slots: Unsafe<Slots<T>> }`, where `Slots<T>` is a private
  intrinsic type, the old arena's insides: a box for each value, which
  stays where it is as more are put in.

## Why it's sound

> Every `&T` points to a value that doesn't change while the reference
> lives, unless the change is to what's in an `Unsafe`; nothing the
> checker's logic reads is in one; and safe code reaches what's in one
> only through a library's `unsafe` block, whose obligation is kept.

- **U1 (opaque to the logic).** `Unsafe`'s field is private to
  `core::cell`, which has no measure or refinement of it. So, as with K1
  (docs/37-cell.md), no measure, refinement, place fact or invariant
  reads what's in one, and a type with one in it, whose own fields a
  library may read in its measures, never reads through it: there's no
  term for what's inside.
- **U2 (invariant).** `as_mut` lends a `&mut T` from a `&`: through it a
  `T` can be put in. So `Unsafe` is invariant in `T`, and then variance
  inference makes every type that holds one inline invariant too, from
  its fields: `Cell`, `Atomic`, `Volatile`, `Mutex`, `Arena`, and a
  library's own (`tests/t/err_soundness_unsafe_wrapper_variance.t`,
  `err_soundness_unsafe_dangling.t`). K2, Y7, V and R3 are this one
  lemma now. A type that holds one behind a `&` has the `&`'s variance,
  which is right: the `&` points to an invariant type.
- **U3 (frozen).** A type is frozen if no `Unsafe` is in it, but behind a
  reference. A box owns what it points to, and an intrinsic type its type
  arguments' values (a `RawBuf<T>` its `T`s), so those are in it. Before,
  `RawBuf` counted as frozen whatever it held, so a `List<Cell<i32>>`
  did: no const could be one anyway, but it was wrong
  (`tests/t/err_const_list_cell.t`).
- **U4 (never `Sync` by structure).** `Unsafe` has no `Sync` impl, and
  only core could add one: an `unsafe impl` is the promise of the package
  that declares the type (docs/41-threads.md). So deriving `Sync` for a
  type with one in it fails, from its fields
  (`tests/t/err_sync_unsafe_derive.t`), and only an `unsafe impl` says
  otherwise, as `Mutex` and `Atomic` do. The compiler's own rule that a
  `Cell` or `Arena` is never `Sync`, whatever its library derives, is
  gone: there's nothing left for it to say.
- **U5 (obligations).** `as_mut`'s caller makes sure nothing else reaches
  what's in it while the `&mut` lives; `as_ref`'s, that no `as_mut`
  reference lives while the `&` does. With them, the safe API of each
  cell keeps K3: `Cell` lends a reference only for one copy or swap,
  `Mutex` only while it's locked (Z2, docs/41-threads.md), `Atomic` and
  `Volatile` never.
- **U6 (arenas).** `Arena::alloc` lends the slots as `&mut` only while it
  runs, and what it gives back is a reference to a value no other has
  been to, in a box of its own, which putting more in doesn't move. Only
  dropping the arena, by value, reaches the values again. The drop check
  (R4, docs/23-arenas.md) sees into `Slots<T>` by its type argument, as
  for any intrinsic type: dropping it may run `T`'s own `discard`, and
  nothing else.

## Arenas stay a little intrinsic

The plan was an arena written wholly in T, `Unsafe<List<Box<T>>>`. That
type-checks and runs, but arenas whose values point to each other, the
reason arenas exist, are then rejected: `List` has a `discard` of its
own, which the drop check can't see into, so dropping the arena might
read the references in its values after what they point to is gone.
Rust has the same problem, and `#[may_dangle]` for it: an `unsafe`
promise that a `Drop` impl only drops its `T`s, never reads them.

That would be a language feature of its own, so the arena keeps a small
intrinsic part instead, `Slots<T>`, whose drop the compiler knows. What
moved out of the compiler is everything that made it a cell: invariance,
not being frozen or `Sync` are its `Unsafe`'s. A library can write a pool
over `Unsafe<List<Box<T>>>` today (`tests/t/arena_user.t`), as long as
its values don't point to each other.

## The intrinsics by name

`src/intrinsic.rs` names every intrinsic once:

```text
pub enum Intrinsic {
    Swap, UnsafeAsMut, UnsafeAsRef, Spin,
    Atomic(AtomicOp), Volatile(VolatileOp), Message(MessageOp),
    Slice(SliceOp), Array(ArrayOp), RawBuf(RawBufOp), Slots(SlotsOp),
    Stream(Stream), ..
}
```

A family of methods is one variant with the method as its operand, so
what runs the family matches on all of them. `Intrinsic::symbol` is the
symbol of its declaration, and `from_symbol` is what the checker asks when
it reads an `#[intrinsic]` function: not one of them is an error there,
as before. The checker records which each declaration is
(`Info::intrinsics`), and monomorphisation hands it on with each copy
(`Executable::intrinsic`). The interpreter and the C backend `match` on
it with no catch-all, so an intrinsic that either of them doesn't run
is a compile error of the compiler's. `IntrinsicType` does the same for the intrinsic
types, which the C backend lays out.

What an intrinsic does that doesn't depend on its type arguments, a
panic, a byte to a stream, the panic message, was already one call of a
function in the C runtime (`t_panic`, `t_write_byte`, `t_msg_push`); what
does, an atomic's width or a slice's element, stays a C template in
`Gen::intrinsic_body`.

The tables that say which nodes a rewrite made, `?` (`Modules::tries`),
format strings (`formats`) and `if let` (`elses`), stay keyed by where
the nodes are, not by node: the borrow checker, which asks most, has
only where a drop or move is. That no two of them share a place is now
checked as they're filled (`Rewrite::made_at`).

## Design decisions worth noticing

- **`as_ref` as well as `as_mut`.** With only `as_mut`, a cell that lends
  `&T`s, as `Once` does, would have to make a `&mut` to make a `&`, an
  obligation stronger than it needs.
- **An `unsafe impl` may need what the type needs.** `Atomic<T>` needs
  `T: Atom`, so its `unsafe impl Sync` must say so, and step 41's rule
  allowed only `Send` and `Sync` there. Now a bound the type declares of
  the parameter is allowed too: every `Atomic<..>` meets it, so a proof
  of `Sync` still needs nothing but `Send` and `Sync` (lemma D1).
- **`Atomic` is `Sync` if `T` is `Send`.** Not `T: Sync`: sharing an
  atomic hands `T` values from thread to thread, as a mutex does. For
  the integers and `bool`, the only `Atom`s, it's the same.
- **The arena's `Slots` over `#[may_dangle]`.** See above: the smaller
  trusted base was not worth a language feature in this step.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| a `#[may_dangle]`-like promise on `Discard` impls | a language feature with its own lemma; only arenas need it so far | — |
| `OnceCell`, `RefCell` in core | can now be written over `Unsafe`; none needed yet | — |
| raw pointers, `Unsafe::get(&self) -> *mut T` | references with obligations do what's needed | — |
| desugar tables keyed by node | their main reader, the borrow checker, has only places | — |

## Exercises

1. Make `Once` above `Sync`. What must its `unsafe impl` need of `T`, and
   why isn't `T: Send` enough?
2. Write `swap(&self, other: &Cell<T>)` for `Cell` over `Unsafe`. What
   goes wrong if `self` and `other` are the same cell, and how does the
   obligation of `as_mut` rule it out?
3. Why is a `struct Pair<T> { a: Unsafe<T>, b: T }` invariant in `T`,
   and a `struct Ptr<'a, T> { r: &'a Unsafe<T> }` too, while a
   `struct Plain<'a, T> { r: &'a T }` is covariant?
4. With `Unsafe<List<Box<T>>>`, which arena programs does the drop check
   reject? Write one, and say what `#[may_dangle]` would promise about
   `List`'s `discard` to accept it.

## Reading

- The Rust standard library, `core::cell::UnsafeCell`: the one primitive
  of Rust's interior mutability, and why every other cell holds one.
- RFC 1327, *Dropck parametricity*: `#[may_dangle]`, the promise this
  step leaves for later.
- Ralf Jung et al., *RustBelt: Securing the Foundations of the Rust
  Programming Language* (POPL 2018): how `Cell`, `Mutex` and the like are
  proved sound, each by its own invariant, over one primitive.
