# Step 37: cells

Code: `core/cell.t` (`Cell<T>`, a struct with one private field, and its
intrinsic `replace`), `Interp::cell_replace` in `src/interp.rs`, the
`Cell` case of `intrinsic_body` in `src/cgen.rs`, the variance of `Cell`
in `src/borrowck/regions.rs`, and `types::frozen` in `src/types.rs`, which
`Checker::const_type` in `src/check/consts.rs` uses.

Until now, nothing behind a `&` changed while the reference lived. That's
what the borrow checker keeps: a value is either shared, `&`, or lent to
one place to change, `&mut`, never both. A *cell* is the exception: a
value that can be changed through a shared reference, as long as nothing
points into it. That's *interior mutability*, Rust's `Cell`.

```rust
use core::cell::Cell;

fn bump(c: &Cell<i64>) with panic {
    c.set(c.get() + 1);
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(1);
    let a = &c;
    let b = &c;
    bump(a);
    bump(b);
    out.line(&a.get())?; // expect: 3
    let old = b.replace(10);
    out.line(&old)?;            // expect: 3
    out.line(&c.into_inner())?; // expect: 10
    Result::Ok(())
}
```

Two shared references, `a` and `b`, change the same cell, one after the
other. What's in a cell is only ever copied out, by `get`, which needs
`Copy`, or moved out, by `replace`, which puts something else in. `set`
is `replace` and then dropping what was there, so it needs `Discard`.
`into_inner` uses the cell up, and `get_mut` lends what's in it, through
a `&mut` to the cell, which is exclusive, as any `&mut` is.

A closure that changes a cell it sees needs only a `&` to it, so it's an
`Fn`, which can be called through a `&` and as often as anyone likes:

```rust
use core::cell::Cell;

fn twice<F: Fn() with panic>(f: &F) with panic {
    f();
    f();
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let calls = Cell::new(0i64);
    twice(&(|| calls.set(calls.get() + 1)));
    twice(&(|| calls.set(calls.get() + 1)));
    out.line(&calls.get())?; // expect: 4
    Result::Ok(())
}
```

## Cycles

An arena (docs/23-arenas.md) lets values point to each other, but only to
what's already in it: each is made with what it points to. With a cell,
one can be changed after the other is made, so two can point to each
other:

```rust
use alloc::arena::Arena;
use core::cell::Cell;

#[derive(Discard)]
struct Node<'a> { v: i64, next: Cell<Option<&'a Node<'a>>> }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a = Arena::new();
    let x: &Node = a.alloc(Node { v: 1, next: Cell::new(Option::None) });
    let y: &Node = a.alloc(Node { v: 2, next: Cell::new(Option::Some(x)) });
    x.next.set(Option::Some(y));
    let mut at = x;
    let mut total = 0;
    for _ in 0..5i64 {
        total = total + at.v;
        match at.next.get() {
            Option::Some(n) => at = n,
            Option::None => {},
        }
    }
    out.line(&total)?; // expect: 7
    Result::Ok(())
}
```

`x` and `y` are shared references, which is all there can be once one
points to the other, and `set` needs no more.

## A struct, with one private field

`Cell<T>` is an ordinary struct, `Cell { value: T }`, whose field is
private to `core::cell`. So it's laid out as a `T` is, and dropping it
drops what's in it, by the drop glue every struct has, and it's `Discard`
when `T` is, by `#[derive(Discard where T: Discard)]`. All of its methods
are T code but one: `replace(&self, value: T) -> T` writes through a `&`,
which no T code can, so it's an intrinsic. The interpreter and the C
backend each write the field of the struct the reference points to.

Outside `core::cell`, the field can't be named: not read, not borrowed,
not matched by a pattern, and no cell made but by `Cell::new`:

```rust,error
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = Cell::new(1);
    let r: &i64 = &c.value; // error: field `value` of `Cell` is private to `core::cell`
    c.set(2);
    out.line(r)?;
    Result::Ok(())
}
```

That `r` would be a `&i64` whose value changes while it lives: the one
thing a cell must never allow.

## Invariant

A `&Cell<&'static str>` can't be used as a `&Cell<&'a str>`, though a
`&&'static str` can be a `&&'a str`. Through the shorter one, `set` would
put in a `&'a str`, and whoever holds the longer one would read it as a
`&'static str`, after what it points to is gone. So `Cell` is *invariant*
in `T`, as `Arena` is (lemma R3 of docs/23-arenas.md): its type must stay
exactly what it was. Variances are worked out from fields, and `Cell`'s
field is a `T`, which would make it covariant, so the borrow checker
fixes it, as it does for intrinsic types.

```rust,error
use core::cell::Cell;

fn keep(c: &Cell<&'static str>, s: &str) {
    c.set(s); // error: lifetime may not live long enough
}

fn main() {}
```

Without it, `keep` would be accepted, by shortening `c` to the lifetime of
`s`; so would this program, which reads `s` after it's gone (in
`tests/t/err_soundness_cell_dangling.t`):

```rust,error
use alloc::string::String;
use core::cell::Cell;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let c: Cell<&str> = Cell::new("long");
    {
        let s = String::from("short");
        c.set(s.as_str());
    } // error: `s` doesn't live long enough
    out.line(&c.get())?;
    Result::Ok(())
}
```

## Frozen

A type is *frozen* if nothing in a value of it can change through a `&`
to it: if no `Cell` is in it, but behind a reference, as what a reference
points to isn't part of the value (`types::frozen`). It's a property of
types, as variance is, worked out from their structure; not a trait, as
no one implements it.

A const must be frozen. Each use of one is a fresh copy, so a cell in it
wouldn't be shared, and would be sound today. But a `&CONST` that lives
for ever, as Rust's do, is one value shared by everyone, which a cell in
it would let anyone change: a global variable no type says is there. So
it's ruled out now, while nothing depends on it.

```rust,error
use core::cell::Cell;

const COUNT: Cell<i64> = Cell::new(0); // error: a const is frozen

fn main() {}
```

A const's initializer may still use a cell of its own: only its value is
kept.

## Why it's sound

> Every `&T` points to a value that doesn't change while the reference
> lives, unless the change is to a cell; and nothing the checker's logic
> reads is in a cell.

- **K1 (opaque to the logic).** `Cell`'s field is private to `core::cell`,
  and nothing there is a measure or a refinement, or reads the field at
  a type the logic has terms for (`T` is a parameter there). So no
  measure, refinement, fact about a place, or struct's invariant reads
  what's in a cell. Lemma M1 (docs/21-measures.md), that a measure is a
  function of the value its `&self` points to, still holds, as what a
  measure reads isn't in a cell; and so do the objects of the logic, and
  P4 (docs/30-postconditions.md). What `get` gives is a new value, which
  no fact ties to the cell, so a call that may change the cell leaves no
  fact to be wrong about (`tests/t/err_soundness_cell_fact.t`).
- **K2 (invariant).** `replace` puts a `T` in through a `&`. With `Cell`
  invariant, the `T` a reference to a cell says is the `T` it holds, for
  as long as the cell lives, so whatever is put in lives long enough for
  everyone who reads it. That keeps P1 (docs/30-postconditions.md) too: a
  call-scoped reference can't be put in a caller's cell.
- **K3 (no reference in).** Nothing gives a `&T` or a `&mut T` into a cell
  from a `&Cell<T>`: what's in it is copied or moved out. `get_mut` needs
  a `&mut Cell<T>`, so nothing else can reach the cell while what it lends
  lives. So the only place that sees a cell change is the cell, and a
  cell's own value is never read but by copying it out whole: changing
  it while something is being read from it can't happen.
- **K4 (linear values).** A value is never made or thrown away by a cell
  but by `set`, which needs `Discard`, and by dropping the cell, which
  needs `Discard` too. `replace` gives the old value back, and
  `into_inner` the last one, to be used up as any value is.
- **K5 (frozen).** A type with no cell in it, but behind a reference, has
  values in which nothing changes through a `&`: by K3, a cell is the
  only thing that can be changed through one.

`set` puts the new value in before it drops the old one. A `discard` that
drop runs may reach the cell, through a reference in the value it drops,
and finds a whole value there, the new one (`tests/t/cell_discard_reentrant.t`).
And no panic unwinds: a cell is never left half changed for someone to
see.

## Design decisions worth noticing

- **A struct, not an intrinsic type.** `Arena` is a fieldless intrinsic
  type, which the interpreter, the C backend and the drop check each know
  of. A cell is a `T` in place, so a struct does all that already, and
  the one thing it can't do, writing through a `&`, is the one intrinsic.
  The price: K1 is kept by `core::cell`'s code, rather than by the
  compiler, as `List`'s invariants are kept by `alloc::list`'s.
- **No `&T` from a `&Cell<T>`.** Rust has `Cell::as_ptr`, for `unsafe`
  code, and `Cell::from_mut`. T has neither yet: everything else in this
  doc leans on K3.
- **Invariance fixed, not inferred.** Variance is worked out from fields,
  and the field doesn't show that `replace` writes it. Rust's `Cell` is
  invariant as its `UnsafeCell` is, which the compiler knows of. T knows
  of `Cell` itself, as there's no `UnsafeCell`.
- **Frozenness, without a trait.** Rust's `Freeze` is an auto trait, which
  T has none of ([docs/29](29-closed-and-sealed-traits.md)); here it's a
  property the compiler works out, as variance is, and nothing can
  implement it or be generic over it.
- **No effect.** Changing a cell isn't an effect: a function can change
  one only if it's given a reference to it, which its signature says.
  A `static`, which needs no reference, is: `global`
  ([docs/38](38-statics.md)).

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `swap(&self, other: &Cell<T>)` | needs a check that the two aren't the same cell | — |
| `Cell::from_mut(&mut T) -> &Cell<T>`, `as_slice_of_cells` | sound, but nothing needs them yet | — |
| `take` | needs a `Default` trait | — |
| `RefCell`: a `&T` into a shared value, checked while running | its own soundness argument, and panics | — |
| threads: a cell is `Send` but not `Sync` ([docs/38](38-statics.md)) | threads | after 39 |
| `&CONST` as a `&'static` | frozenness is ready for it | — |

## Exercises

1. Make `Cell` covariant again (in `src/borrowck/regions.rs`), and run
   `tests/t/err_soundness_cell_dangling.t`. What does the interpreter say?
2. Add `fn get_ref(&self) -> &T` to `Cell`. Write a program in which a
   `&i64` changes while it lives, and one in which the refinement checker
   proves something false.
3. Write `set` the other way: move the old value out, drop it, then put
   the new one in (you'll need an intrinsic). What can a `discard` that
   reaches the cell see, and what would `get` give it?
4. Write a `Counter` with a `Cell<u64>` and `fn next(&self) -> u64 with
   panic`. Why can't `next` be a measure?
5. Why is `&Cell<i64>` frozen, while `Box<Cell<i64>>` isn't?

## Reading

- The Rust standard library's `std::cell` docs: `Cell`, `RefCell` and
  `UnsafeCell`, and why each is sound.
- Ralf Jung et al., *RustBelt: Securing the Foundations of the Rust
  Programming Language* (2018): `Cell`'s soundness proved, with the
  library's `unsafe` code, in a separating logic.
- The Rust Reference, *Interior mutability* and *Constant promotion*:
  what may change behind a `&`, and why promoting a `&CONST` needs a type
  without `UnsafeCell` in it.
- Peter W. O'Hearn, John C. Reynolds and Hongseok Yang, *Local Reasoning
  about Programs that Alter Data Structures* (2001): why a logic must know
  what can change behind a pointer.
