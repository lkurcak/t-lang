# Step 38: statics, atomics, `Send` and `Sync`

Code: `core/sync.t` (`Send`, `Sync`, `Atom` and `Atomic<T>`), the marker
cases of `Traits::implements_at` in `src/traits.rs` and of
`Checker::needs` in `src/check/traits/derive.rs`, `Checker::static_type`
in `src/check/consts.rs`, the `Res::Static` case of `Checker::lookup` in
`src/check/paths.rs`, the static case of `Builder::place` in
`src/borrowck/build.rs` (`Const::Static`), `Interp::static_place` and
`Interp::atomic` in `src/interp.rs`, and `Gen::static_name` and the
`Atomic` case of `intrinsic_body` in `src/cgen.rs`.

Every function so far could change only what it was given: its
arguments, and what they reach. A *static* is a place every function can
reach without being given it, for the whole run: a global variable. This
step adds them, and the three things that make them sound before there
are threads: what's in a static must be safe to share (`Sync`), the only
way to change it is atomically (`Atomic`), and a function that uses one
says so (`global`).

```rust
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn bump() -> u32 with global { COUNT.fetch_add(1) }

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&bump())?;       // expect: 0
    out.line(&bump())?;       // expect: 1
    out.line(&COUNT.load())?; // expect: 2
    Result::Ok(())
}
```

## Atomics

An `Atomic<T>` is like a `Cell<T>` (docs/37-cell.md): it changes through a
`&`, and nothing ever points into it. What's different is how: each
change is one step of the machine, which no other thread sees half of,
and all threads see all changes in one order, the same for all: they're
*sequentially consistent*. There are no weaker orderings, which are where
lock-free code goes wrong, quietly, on some machines and not others.

```rust
use core::sync::Atomic;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = Atomic::new(250u8);
    out.line(&a.fetch_add(10))?; // expect: 250
    out.line(&a.load())?;        // expect: 4
    match a.compare_exchange(4, 5) {
        Result::Ok(old) => out.line(&old)?, // expect: 4
        Result::Err(_) => {},
    }
    out.line(&a.swap(9))?;      // expect: 5
    out.line(&a.into_inner())?; // expect: 9
    Result::Ok(())
}
```

Only what the machine reads and writes in one step can be atomic: the
integers and `bool`, which implement `Atom`, a sealed trait. Adding and
subtracting wrap around, as `wrapping_add` does, for integers alone.
`fetch_and`, `fetch_or` and `fetch_xor` work on `bool` too. The
intrinsics are `load`, `swap`, `compare_swap` (which gives back what was
there; whether it changed is whether that's what was expected) and the
five `fetch_`s; `store` and `compare_exchange` are written with them. In
C, each is one of the compiler's `__atomic` builtins, with
`__ATOMIC_SEQ_CST`.

## `Send` and `Sync`

A type is `Sync` if its values may be shared by threads, through `&`s:
nothing in one changes through a `&`, or only atomically. It's `Send` if
its values may be handed to another thread, to be used there alone.
They're the two questions any sharing asks, and a static is shared by
every function there is, so its type must be `Sync`:

```rust,error
use core::cell::Cell;

static COUNT: Cell<u32> = Cell::new(0); // error: a static must be `Sync`

fn main() {}
```

Like `Copy` and `Discard`, they're *markers*: only ever derived or built
in, never written, so whether a type is one follows from its fields
alone. The built-in ones:

| Type | `Send` if | `Sync` if |
|---|---|---|
| integers, `bool`, `char`, `()`, `str`, functions | always | always |
| `&T` | `T: Sync` | `T: Sync` |
| `&mut T` | `T: Send` | `T: Sync` |
| `Box<T>`, `[T; N]`, `[T]` | `T: Send` | `T: Sync` |
| a closure | what it captures is | what it captures is |
| `Cell<T>`, `Arena<T>` | `T: Send` | never |
| `Atomic<T>` | always | always |

`&T: Send` needs `T: Sync`: a reference handed over shares what it points
to. So a derived `Send` can need `Sync` of a type parameter:

```rust
use core::cell::Cell;

#[derive(Discard, Send)]
struct Holder<'a, T> { r: &'a T }

fn send<T: Send + Discard>(_t: T) {}

fn main() {
    let n = 3u8;
    send(Holder { r: &n });
    let c = Cell::new(4u8);
    send(c); // a cell may move to another thread, as it isn't shared
}
```

`send(Holder { r: &c })` would fail: `Holder<Cell<u8>>` isn't `Send`, as
that needs `Cell<u8>: Sync`. An exported struct with fields that aren't
says what its derive needs, `#[derive(Send where T: Sync)]`, as it does
for `Discard` (docs/24-visibility.md). Rust computes `Send` and `Sync` for
every type, as *auto traits*: a struct is `Sync` unless a private field
isn't, and changing that field quietly changes what the type promises,
which a package that uses it may count on. T's are only what's derived,
and what's derived only follows the fields (lemma D1 of
docs/17-retrospective-1.md): there's no way yet to say a type is `Sync`
against its fields, which a `Mutex` will need, with `unsafe`.

## Statics

`static NAME: T = value;` declares one, at a module's top level. Its
value is worked out while compiling, as a const's is (docs/36-consts.md):
the initializer may panic, which is then an error, and do nothing else;
it's run at 32 and 64 bits, which must agree. So there's no order to
start statics in, and nothing to go wrong at start-up. Its type must be:

- not *frozen* (lemma K5 of docs/37-cell.md): a static whose value never
  changes is what a const is for;
- plain data, as a const's is, with atomics in it: nothing on the heap,
  no reference but to a literal;
- `Sync`, as every function can reach it;
- `Discard`, as it's never used up.

A use of a static's name is the place it is: what a `&'static` to it
points to. So `&COUNT` is a `&'static Atomic<u32>`, and its frozen fields
can be read as any value's. But there's no `&mut` to a static, no moving
out of one and no assigning to one: the borrow checker treats it as
behind a `&`, which it is.

```rust
use core::sync::Atomic;

#[derive(Discard, Sync)]
struct Stats { name: &'static str, limit: u32, hits: Atomic<u32> }

static STATS: Stats = Stats { name: "hits", limit: 2, hits: Atomic::new(0) };

fn hit() -> bool with global {
    let s: &'static Stats = &STATS;
    s.hits.fetch_add(1) < s.limit
}

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&hit())?;      // expect: true
    out.line(&hit())?;      // expect: true
    out.line(&hit())?;      // expect: false
    out.line(&STATS.name)?; // expect: hits
    Result::Ok(())
}
```

```rust,error
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn main() with global {
    let n = COUNT.get_mut(); // error: cannot borrow `COUNT` as mutable
    *n = 3;
}
```

## The effect `global`

Each use of a static's name has the effect `global`, which a function
says, `with global`, if it's exported, or has inferred otherwise
(docs/24-visibility.md). So a signature says whether a function may read
or change state it isn't given, and `with {}` means it can't:

```rust,error
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

export fn count() -> u32 { COUNT.load() } // error: `COUNT` is a static, but `count` doesn't say `with global`

fn main() with global {
    let _ = count();
}
```

A closure that uses a static has `global` too, which a bound must allow,
`F: Fn() -> u32 with global` or an effect parameter. A closure given a
`&` to a static doesn't: what it changes is reached through what it
captured, as with any reference. Neither a const's initializer nor a
static's may have `global`: what's in a static is only known as the
program runs.

The interpreter makes a static's slot the first time it's used, from its
value; the C backend makes a global, and sets it before `main` runs.

## Why it's sound

> Whatever a `&` points to doesn't change while the reference lives,
> unless it's in a cell or an atomic; a value shared between threads has
> no cell in it; and a function without `global` (or `unsafe`) touches
> only what its arguments reach.

- **Y1 (what changes through a `&`).** Only three things are written
  through a `&`: a cell, by `Cell::replace`; an arena, by `Arena::alloc`;
  an atomic, by its intrinsics. Each is opaque to the checker's logic
  (K1 of docs/37-cell.md; `Atomic`'s field is private to `core::sync`,
  which has no measures). The first two are never `Sync`: their libraries
  don't derive it, and the checker refuses it for them whatever's
  derived. An atomic's writes are atomic. So any `&` to a `Sync` value,
  in any number of threads, reads nothing another changes but atomically:
  no data race.
- **Y2 (markers follow structure).** A derived `Send` or `Sync` needs only
  `Send` or `Sync` of its fields, and so of its type parameters; a
  closure's are its captures'; an opaque type has only what its bounds
  say. So by induction on types, `T: Sync` means every part of a `T`
  reached through a `&` is a frozen value, an atomic, or behind a `&` to
  a `Sync` value, and `T: Send` means the same of everything a `T`
  shares.
- **Y3 (statics are `Sync`).** Any function can reach a static, from any
  thread, or from an interrupt handler, which freestanding code will
  have: it's shared by all of them, so Y1 needs its type `Sync`.
- **Y4 (a static is behind a `&`).** A use is `*S`, `S` a `&'static T`.
  The borrow checker allows no `&mut` to it, no move out of it, no
  assignment: so its frozen parts never change, and the rest are atomics
  (statics' types are plain data with atomics, and `Sync`, so no cell).
  Measures (M1 of docs/21-measures.md), facts about places, and struct
  invariants read only frozen parts, which hold for the whole run. A
  static holds no reference but to a literal (plain data), so P1
  (docs/30-postconditions.md) holds: no call-scoped reference gets into
  one.
- **Y5 (`global`).** A function whose effects have no `global` and no
  `unsafe` names no static, nor calls anything that does, so what it
  reads and changes is reached from its arguments. Const and static
  initializers can't have `global`, so evaluating them still needs
  nothing but the interpreter (C1 of docs/36-consts.md).
- **Y6 (start-up).** A static's value is worked out while compiling, as a
  const's is, at both widths, so it's the same on every machine, and
  setting it can't panic or depend on another static. A static is never
  dropped, like Rust's: as its type is `Discard`, no value that must be
  used up is lost (docs/06a.1-linear-types.md). A `discard` of one with
  something to do never runs.
- **Y7 (atomics).** Only `Atom` types can be atomic, the integers and
  `bool`, which have no references in them; `Atomic` is invariant anyway,
  like `Cell`, not frozen, never `Copy`, and only copies come out of it.
  K1 to K4 of docs/37-cell.md hold of it as of a cell, with "one step"
  for "one thread".

## Design decisions worth noticing

- **One `Atomic<T>`, not `AtomicU32` and nine others.** Generic code over
  integer widths gets atomics for free, and there's one set of
  intrinsics, as Rust is moving to with its `Atomic<T>`.
- **Sequentially consistent only.** Acquire/release and relaxed orderings
  are faster on some machines, and wrong in ways that don't show on
  others. They can come later, behind `unsafe`, when a lock needs them.
- **Statics must change.** A static of a frozen type is refused, with a
  fix that makes it a const: a static is where a program keeps state, and
  a table that never changes is a const, which `&CONST` will make cheap.
- **No `static mut`.** Rust's is `unsafe` to touch; here, a static changes
  only through what's in it, so every use is safe.
- **`global` on every use.** Even reading a frozen field of a static is
  `global`: simpler, and a frozen value is a const's job.
- **Initialised by the compiler, set by C at start-up.** C could put the
  values in the program's data, which a freestanding target will want;
  for now `main` sets them, from the same value bodies the interpreter
  runs.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| threads, `spawn`, `Mutex` | `Send`/`Sync` were the groundwork | after 39 |
| `unsafe impl Sync` against the fields | its syntax comes with `Mutex` | after 39 |
| weaker orderings | SeqCst is the safe default | — |
| `&STATIC` in a static's initializer | needs addresses in const evaluation | — |
| statics in C's data section | the freestanding target | 39 ✅ |
| `&CONST` promotion, frozen statics | frozenness is ready for it | — |

## Exercises

1. Make `&T: Send` need `T: Send` (in `Traits::implements_at`), and write
   a program that would race two threads, if there were threads.
2. Why must a static's type be `Discard`, but a const's needn't?
3. Write a counter of unique ids with a static, and a function `fresh()
   -> u64 with global`. Why can't it be a measure?
4. A function takes a `&'static Atomic<u32>` and changes it, without
   `global`. Why is that fine?
5. Why does `Atomic` need no `Copy` bound on `T` for `load`?

## Reading

- The Rust standard library's `std::sync::atomic` docs, and the Rustonomicon's
  chapters *Send and Sync* and *Atomics*.
- Hans-J. Boehm and Sarita V. Adve, *Foundations of the C++ Concurrency
  Memory Model* (2008): sequential consistency for data-race-free
  programs.
- Leslie Lamport, *How to Make a Multiprocessor Computer That Correctly
  Executes Multiprocess Programs* (1979): what sequential consistency is.
- Daan Leijen, *Koka: Programming with Row Polymorphic Effect Types*
  (2014): state as an effect in a function's type.
