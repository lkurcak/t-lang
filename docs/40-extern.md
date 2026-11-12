# Step 40: `extern fn` and volatile memory

Code: `src/check/externs.rs` (`Checker::extern_signature` and
`Checker::externs_known`), the `extern` case of `Parser::item` in
`src/parser.rs`, `core/volatile.t` (`Volatile`), `Gen::import_body` and
`Gen::exports` in `src/cgen.rs`, the `Volatile` case of
`Gen::intrinsic_body`, and the import and `Volatile` cases of
`Interp::call` and `Interp::intrinsic` in `src/interp.rs`.

Step 39 made programs for a machine with no operating system, but the
machine was still reached only through C: the five platform functions,
written in C, and nothing in T could touch a device. This step adds the
two things T was missing to do that itself:

- **`extern fn`**: functions with C's calling convention, under their own
  names. One with no body is an *import*, a C function T calls; one with a
  body is an *export*, a T function C can call.
- **`core::volatile::Volatile<T>`**: a value at a fixed address, a
  device's register, read and written by *volatile* accesses, each one
  done exactly when and as often as the program says.

With them a program can write the platform's `t_platform_write` and
`t_platform_halt` in T, so a freestanding program that doesn't allocate
needs no C at all, only the start-up code that calls `t_main`.

## Imports

```rust
extern fn putchar(c: i32) -> i32;

fn main() {
    // SAFETY: `putchar` is the C library's, and only writes a byte.
    let _ = unsafe { putchar(104) };
    // error: `putchar` is a C function, which the interpreter can't call
}
```

An import declares a C function: its name, which is its C name, and its
type. What it does is C's, which the checker can't see, so a call is
`unsafe` (docs/09-unsafe.md): its caller promises that a C function by
that name exists, takes and gives what the import says, and does only
what the import says. That includes its effects. An import that may call
back into T code that panics says `with panic`, and its callers have
`panic` too, as `unsafe` blocks take away only `unsafe`:

```rust,error
extern fn may_panic() with panic;

fn quiet() with {} {
    // SAFETY: `may_panic` does only what it says.
    unsafe { may_panic() }
}
// error: `may_panic` may panic, but `quiet` doesn't say `with panic`
```

The interpreter has no C library and no machine, so it stops where an
import is called, as above. `t build` compiles the call. That program
built for the hosted target prints `h`.

An import that reads or writes state that outlives the call, C's statics
or a device's, says `with global`, as using a static of T's does: two
calls of it may give different things, or one may change what the next
gives. A heap's count of bytes in use, kept by the platform's C, is read
by an `extern fn heap_used() -> usize with global;`.

## Exports

```rust
extern fn register(x: u32) -> u32 { x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&register(6)) // expect: 7
}
```

An export is an ordinary T function, which T can call as any other, and
which `t build` also makes a C function by its name: `register` here,
though that's a word of C's. C may call it from anywhere: a callback, an
interrupt, start-up code. It's never reported as unused.

What C promises is nothing T can check. So an export can't say `with
unsafe`, which would make its obligations its caller's, as `main` can't:
C calls it, and keeps none. Nor can its parameters be refinements, as C
proves nothing of what it passes:

```rust,error
extern fn halve(x: { x: u32 | x % 2 == 0 }) -> u32 { x / 2 }
// error: an exported `extern fn` takes no refinement
```

Nor can it allocate. C may call it from an interrupt, which can come in
the middle of an allocation, and the platform's allocator, as most, can't
be started again before it's done. So an export's effects, said or
inferred, are at most `global` and `panic` (step 71,
docs/71-t-metal-findings.md):

```rust,error
use alloc::list::List;

extern fn on_interrupt() {
    let mut l = List::new();
    l.push(1u8);
}
// error: an export can't have `alloc`
```

## What crosses

Only C's scalars: the integer types, `bool`, and as a result `()`
(C's `void`) and `!` (a `_Noreturn` function). `usize` and `isize` are
`uintptr_t` and `intptr_t`, so an address crosses as a `usize`. A
reference, a struct, an enum, a `char` can't:

```rust,error
extern fn put(s: &str);
// error: an `extern fn` takes only integers and `bool`s, which are what C's are, not `&str`
```

T doesn't say how it lays out anything else (docs/33-c-backend.md), and
won't promise C a layout before there's a way to ask for one. A `char` is
a `u32` that's a Unicode scalar value, which no C type says. An import
can still ask its callers for more than C does, with refinements on its
parameters, which they prove, but its result is only what its C type
says.

An `extern fn` has no generic parameters, no lifetimes and no effect
parameters: C calls it, or it's C, as it is. Nor can it be a `#[test]`,
`#[intrinsic]`, `#[measure]` or `#[blame_caller]`.

## Names

A C name is one for the whole program: two `extern fn`s can't share one,
and only a program, a package that starts in `main.t` (or a single
file), can have them, not a library:

```text
error: only a program can have an `extern fn`, not a library: a C name is one
       for the whole program, which no library in it could be sure is its own
```

Otherwise two libraries that didn't know of each other could each import
the same C function with a different type, or export the same name, and
adding a dependency would break a build
(docs/29-closed-and-sealed-traits.md). Whether a package is a program is
its own business, so a library fails the same way checked alone as in a
program (lemma P2 of docs/26-packages.md). A driver can still be a
library: one whose functions take what they need, the addresses of its
registers say, from the program that imports the C functions.

The names `main` and those starting with `t_` are the runtime's. C
names don't clash with C's own words, or with what a header of the C
library declares: each import is called through a C name of its own,
given the import's symbol by an `__asm__` label, as is each export.

## Volatile memory

A device is memory the program doesn't own, at addresses the machine
says. `Volatile<T>` is a `T` at one of them:

```rust
use core::volatile::Volatile;

/// The transmit register of the board's UART.
const UART_TX: usize = 0x1000_0000;

fn uart() -> &'static Volatile<u8> {
    // SAFETY: the board has a UART whose transmit register, a byte, is at
    // `UART_TX`, and nothing else uses it.
    unsafe { Volatile::at(UART_TX) }
}

fn put(s: &str) {
    for b in s.as_bytes().iter() { uart().write(*b) }
}
```

`Volatile::at(address)` is the only way to get one, and it's `unsafe`:
the caller promises the address is aligned for a `T`, there's one there
for the whole run, and nothing but volatile accesses of a `T` touch it. No
value of the program's is there. After that, `read` and `write` are safe.
Each is one access, never left out, merged with another or moved past
another, which is what a device needs: writing a byte to a UART twice
sends it twice.

`T` is an `Atom`, an integer or `bool`, as for `Atomic`
(docs/38-statics.md): what the machine reads or writes in one access. A
`Volatile` is like a `Cell` (docs/37-cell.md): it changes through a `&`,
what's there is only copied in and out, and the checker's logic never
looks inside. That matters more here, as the device changes it on its own:

```rust,error
use core::volatile::Volatile;

fn one(_x: { x: u8 | x == 1 }) {}

fn f(r: &Volatile<u8>) {
    r.write(1);
    one(r.read());
}
// error: can't prove `x == 1` for argument `_x` of `one`
```

It's invariant in `T`, not frozen, never `Copy`, and neither `Send` nor
`Sync`. Two threads, or a program and its interrupt handler, using one
register need more than volatile accesses to agree on it. That's a
lock's job, which comes with threads.

## The platform in T

The runtime calls `t_platform_write` now a byte at a time, so its
parameter is a scalar:

```c
void t_platform_write(uint8_t byte);
_Noreturn void t_platform_halt(void);
```

A program built for the freestanding target can write either, or both,
as exports with exactly those types. Here's a platform for a board with
a UART, and a power controller that turns the board off when it's
written to:

```rust
use core::volatile::Volatile;

const UART_TX: usize = 0x1000_0000;
const POWER: usize = 0x0010_0000;

extern fn t_platform_write(b: u8) {
    // SAFETY: the UART's transmit register is at `UART_TX`, and only this
    // writes it.
    let tx: &'static Volatile<u8> = unsafe { Volatile::at(UART_TX) };
    tx.write(b)
}

extern fn t_platform_halt() -> ! {
    // SAFETY: the power controller's register is at `POWER`.
    let power: &'static Volatile<u32> = unsafe { Volatile::at(POWER) };
    loop { power.write(0x5555) }
}
```

They may use statics, as a device's state can be in one, but nothing
else: the runtime calls them while it panics, or because memory ran out,
so a panic or an allocation in one would never end.

```text
error: the platform's `t_platform_write` may only use statics: the runtime calls it
       while it panics, or runs out of memory, so it can't have `panic`
```

On the hosted target the platform is the C library's, so a program that
writes either doesn't run there, nor build for it. It checks the same,
though, and its tests, each a hosted program of its own, call them as the
T functions they are (docs/71-t-metal-findings.md). The other three, the
memory, stay C's: the runtime
trusts the block `t_platform_alloc` gives back to be free memory of the
right size, which is a promise no safe T function could make. Writing
them in T needs a type for such a block, made only by `unsafe` code,
which belongs with allocators.

## Why it's sound

> An `extern fn` crosses only values every bit pattern of whose C type is
> a value of its T type; what C does is an obligation of the `unsafe`
> code that calls it; and a volatile value is a cell the machine changes.

- **E1 (imports).** A call has the import's declared effects and
  `unsafe`. Its obligation is that the C function exists with that C type
  and does only those effects, touching no memory of the program's but
  through what it's given, which is only scalars. Kept, the call is like
  a call of a T function with that signature, and every argument for
  effects (Y5 of docs/38-statics.md, docs/07-effects.md) goes through.
  A refinement on its result would be a fact C was trusted to prove, so
  there's none.
- **E2 (exports).** An export is a T function checked as any other, so
  whoever calls it gets what its type says, given arguments of its
  parameters' types. C passes scalars, and every bit pattern of a C
  integer is a value of the T integer of its width (a `bool`'s being 0 or
  1 is the platform's contract, as in docs/39-freestanding.md, F2). Its
  parameters have no refinements, and it has no `unsafe`: nothing it
  counts on is anyone's to keep. Being called from C in the middle of T,
  from a callback or an interrupt, it shares with the code it interrupts
  only statics, which are `Sync` and change only atomically (Y1, Y3). The
  heap is the runtime's one other shared state, and an export never
  touches it: its effects are at most `global` and `panic`, so it never
  allocates (effects bound what a body does, docs/07-effects.md), and it
  frees nothing, as it owns nothing on the heap: it's given only scalars,
  and statics hold only plain data (docs/38-statics.md).
- **E3 (layout).** No reference, struct or enum crosses, so no
  representation of T's is promised to C, and the backend stays free to
  lay values out as it likes.
- **E4 (names).** All `extern fn`s are in the program's package, as no
  library can have one, so whether two clash is a local fact of it, and
  no dependency added later can make it so. Each C name is the program's alone, and symbols, not
  C declarations, carry them, so neither C keywords nor the C library's
  headers can change what an import calls or an export defines.
- **V (volatile).** `at` is `unsafe`, with the obligation that there's a
  `T` at the address for the whole run, which nothing of the program's
  is, and that only volatile accesses of a `T` touch it: so the
  `&'static Volatile<T>` it gives back points to a `T` as long as it
  lives, and is no alias of anything the borrow checker tracks. `read`
  and `write` copy a `T` out and in; nothing points into it, and the
  checker's logic doesn't read it (it's opaque, as a `Cell` is, K1 of
  docs/37-cell.md), so no fact about it is ever stale. It's invariant
  and not `Sync`, so nothing reaches it from another thread.
- **P (platform).** The platform's functions written in T are exports,
  so E2 holds of them; their types are the runtime's, checked, so the
  runtime's C calls them as it declares them; `t_platform_halt` gives
  back `!`, so it returns only if its body does what a `!` can't. Their
  effects are at most `global`, so what the runtime calls while
  panicking doesn't panic, and doesn't allocate.
- **I (the interpreter).** It runs no import and names no address: it
  stops there, with an error, and runs nothing it can't check.

## Testing it

The rules are tested as the language's others are, in `tests/t`
(`*extern*` and `*volatile*`) and `tests/freestanding` (the platform's
functions). What only a machine runs is in `tests/extern.rs`: each
`hosted_*` program is built and run against the C library, and each
`machine_*` one is built for the freestanding target and linked with
`tests/shared/machine.c` (and the case's own `.c`, if any), a machine with a device, a screen of bytes with
a cursor, which the program drives with volatile accesses from its own
`t_platform_write`, and which C calls back into. The differential tests
(`tests/compiled.rs`) leave out a program where the interpreter stops for
want of a machine.

## Design decisions worth noticing

- **One keyword, both ways.** `extern fn` with no body is C's, with a
  body is T's that C may call, as a bodiless `fn` in a trait is the
  impls'. Rust says `extern "C" { fn .. }` and `#[no_mangle] extern "C"
  fn`; there's one calling convention here, and no mangling to turn off,
  as an export is always under its name.
- **Scalars only.** C structs need `#[repr(C)]`, and pointers need a
  story for what they point to; both are their own steps. Addresses as
  `usize` are enough to reach devices, and `Volatile` is enough to use
  them.
- **A handle, not raw pointers.** T has no raw pointers, and this step
  doesn't add them: `Volatile::at` makes the one promise once, where the
  address is named, and every access after is safe. A `read_volatile` of
  an address would be `unsafe` at every access.
- **The byte-wise write hook.** Writing a panic's message a byte at a
  time costs nothing that matters, and keeps the platform's function a
  scalar one, so T can write it without pointers.
- **Root package only.** It's a restriction that can be lifted: a later
  design could let a library own the C names it declares, if the program
  says it may.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `#[repr(C)]` structs, pointers across C | needs a layout promise and a story for what pointers point to | — |
| the memory platform functions in T | needs an `unsafe`-made block type | with allocators |
| `extern fn` in libraries | a C name is the whole program's | — |
| interrupt handlers' own rules, beyond not allocating | `Volatile` isn't `Sync`; needs locks and threads | after threads |
| allocating in an export C calls back synchronously | needs a way to say it's no interrupt's | — |
| `Volatile` across threads | sharing a device needs a lock | after threads |

## Exercises

1. Write a `t_platform_write` for a UART that has a status register: wait
   until its "ready" bit is set before each byte. Why must both be
   volatile, and what would go wrong if the status were read once?
2. Why can an import's parameters be refined, but not its result? Why is
   it the other way round for an export?
3. Make `Volatile` `Sync`, and write a program with an interrupt handler
   (an export) and a `main` that would both use a register. What could
   go wrong?
4. Why can't a library have an `extern fn`? Find two libraries that
   would each check alone, and break when one program uses both, if they
   could.
5. Why does an import's call have `unsafe` even when it says `with {}`?

## Reading

- The Rust reference, *External blocks* and *The `no_mangle` attribute*;
  the Rustonomicon, *FFI*.
- The Rust standard library's `core::ptr::read_volatile` and
  `write_volatile`, and the `volatile-register` and `vcell` crates: a
  register as a type.
- ISO/IEC 9899:2011 (C11), §6.7.3: what `volatile` means, and §5.1.2.3:
  that volatile accesses are what a program observably does.
- John Regehr, *Volatile Structures* and Eide and Regehr, *Volatiles Are
  Miscompiled, and What to Do about It* (EMSOFT 2008).
- The GCC manual, *Asm Labels*: a C declaration's symbol, given.
