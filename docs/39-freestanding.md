# Step 39: the freestanding target

Code: `src/target.rs` (`Target`), `Loader::manifest` in `src/load.rs`
(what a target provides), the target case of `Resolver::modules` in
`src/check/modules.rs`, `Checker::main_signature` in `src/check.rs`,
`src/cgen/shim.h` (the runtime, on the platform's functions),
`src/cgen/hosted.h` (those functions on a C library), and
`Gen::static_init` and `Gen::main` in `src/cgen.rs`.

T is meant to run where there's no operating system: a kernel, a
bootloader, firmware. Until now `t build` made programs for Linux and
macOS only. The runtime it compiled in wrote panics to `stderr` with
`stdio`, took memory from `malloc`, stopped with `_Exit`, and set the
statics in C's `main`. None of that exists on a bare machine. This step
adds a second *target*, `freestanding`, for a machine with no C library
and nothing around `main` but what the program's *platform* gives it:

```text
$ t build --target freestanding kernel.t
$ ls
kernel.o  kernel.t
```

The output is an object, `kernel.o`, which exports one function,
`void t_main(void)`, that runs T's `main` and returns. The platform's
start-up code calls it, and links the object with the five functions the
runtime needs from the machine.

## What a target provides

A target is a property of a build, not of a package: the same program
can be built for both. The package already says what it needs, in its
`t.toml` (docs/35-libraries.md); the target says what's there:

| Target | Libraries | `main` | What `t build` makes |
|---|---|---|---|
| `hosted` (the default) | core, alloc, std | `fn main()` or `fn main(os: Os)`, `()` or `Result<(), E>` | a program |
| `freestanding` | core, alloc | `fn main()`, giving back `()` | an object with `t_main` |

std is the operating system's: its streams, the `Os` handed to `main`,
and the report of the error `main` gives back. So it's not there on a
freestanding target, and a `t.toml` that lists it, the program's or a
dependency's, is an error there:

```text
error: `std` is for the hosted target, and this is built for the
       freestanding one
 --> t.toml:1:24
  |
1 | libraries = ["alloc", "std"]
  |                       ^^^^^
```

A package with no `t.toml` has its target's libraries, so a program in
a single file that needs no std builds for either target unchanged. One
that names `std::` gets the same error, at the path. And a `main` that
gives back a `Result` has nothing to report its error:

```text
error: on the freestanding target, `main` gives back `()`: what reports
       an error it gives back is std's, which a freestanding program
       hasn't got
```

Checking is otherwise the same on both targets, as it's the same for
every machine (docs/11-integers.md): the target decides which libraries
a program has, and nothing else. A program that writes the platform's
functions in T (docs/40-extern.md) is checked the same way too, and
tested on the hosted target; it's running it there, or building it for
it, that fails, as the platform there is the C library's
(docs/71-t-metal-findings.md).

## The platform

What the runtime needs from the machine is five C functions:

```c
void t_platform_write(uint8_t byte);
_Noreturn void t_platform_halt(void);
void *t_platform_alloc(size_t size, size_t align);
void *t_platform_realloc(void *p, size_t size, size_t align, size_t new_size);
void t_platform_free(void *p, size_t size, size_t align);
```

- `t_platform_write` writes a byte of a panic's message wherever errors
  go: a serial port, a screen, nowhere. It can't fail, or it fails
  quietly. (Until step 40, it took a pointer and a length:
  docs/40-extern.md.)
- `t_platform_halt` stops the program after a panic, and never returns:
  a loop, a reset, a shutdown.
- `t_platform_alloc` gives a block of `size` bytes, never 0, aligned to
  `align`, a power of two no bigger than `_Alignof(max_align_t)`, or
  `NULL` when there's no memory, which is `alloc`'s abort
  (docs/16-out-of-memory.md). `realloc` and `free` get back the size and
  alignment the block was given with, as Rust's `GlobalAlloc` does, so a
  simple allocator needs no headers.

A program that doesn't allocate never calls the last three, and links
without them. As with any C compiled freestanding, the platform also
provides `memcpy`, `memmove`, `memset` and `memcmp`, which C compilers
call on their own, and its compiler's support library (`libgcc`,
`compiler-rt`) if the machine needs one.

The hosted target is the same runtime with these five written over the
C library (`cgen/hosted.h`), plus std's streams and a C `main`. So a
program built for either target runs the same T. Every rule of arithmetic,
every check, every drop and every panic's message is in the runtime they
share (`cgen/shim.h`), which needs only C11's freestanding headers
(`stdbool.h`, `stddef.h`, `stdint.h`). Numbers in panic messages, which
used `printf`, are now written by the runtime itself.

Here's a program that builds for both. It needs nothing of std, so it
shows what it found the only way a freestanding program can, by how it
ends:

```rust
use alloc::list::List;

fn main() {
    let mut squares = List::new();
    for i in 1..11u32 { squares.push(i * i) }
    let mut sum = 0u32;
    for s in squares.iter() { sum = sum + *s }
    core::panic::panic_with(&sum) // error: panicked: 385
}
```

## Statics in the program's data

A static's value is worked out while compiling (docs/38-statics.md), so
it doesn't need code to set it: it's now a C constant initializer, in
the program's data, on both targets. C's `main` used to set each static
before T's `main` ran. On a bare machine nothing should have to run
first: an interrupt can come before `t_main` does, and its handler can
use a static.

```c
static T_Atomic_u32 s_COUNT = {((uint32_t)UINT64_C(7))};
static T_Config s_CONFIG = {{t_lit0, 3}, {{{.tag = 0}, {.tag = 1, .u.v1 = {((uint32_t)UINT64_C(3))}}}},
    {((uint64_t)UINT64_C(0))}, {{((uint8_t)UINT64_C(1)), ((uint8_t)UINT64_C(2)), ((uint8_t)UINT64_C(3))}}};
```

That's `tests/t/freestanding_statics.t`'s `COUNT` and `CONFIG`: a string,
an array of two enums, an atomic and an array of bytes.

`Gen::static_init` reads the static's value body, the parts built one
by one into temporaries (`consts::value_body`), and writes each as an
initializer: a struct's fields in braces, a variant's tag and its fields
by name, a string as its literal's address and length, which are an
address constant.

## Why it's sound

- **F1 (libraries).** A target provides libraries, and a build fails
  unless every package of the program lists only those. By L3 of
  docs/35-libraries.md, the code a program reaches comes from the
  libraries its packages list, so everything a freestanding program
  calls is there: no std intrinsic is ever compiled for it.
- **F2 (one runtime).** What T means, the arithmetic, the checks, drop
  glue, panic messages, is written once, in the runtime both targets
  share, and in the C translated from the MIR (docs/33-c-backend.md). The
  targets differ only in the platform's functions, whose contracts are
  above. So every hosted program the differential tests compare with the
  interpreter also tests the freestanding runtime. A platform that
  breaks its contract has undefined behaviour, as `unsafe` code that
  breaks its obligations does.
- **F3 (statics as data).** The initializer is a constant expression of
  the same value the C `main` used to assign, worked out at both widths
  (Y6 of docs/38-statics.md), so the program sees the same statics, and
  sees them before any of its code runs.
- **F4 (entry).** A freestanding `main` gives back `()`, so there's no
  error to report and nothing to drop after it. `t_main` returns to the
  platform, which decides what happens next.
- **F5 (no C library).** The freestanding C includes only C11's
  freestanding headers, and the object needs no symbol but the
  platform's and `mem*`. Both are checked by the tests, for every
  program that needs no std.

## Testing it

`tests/compiled.rs` builds every program of the tests, examples and
docs that needs no std a second time, for the freestanding target. It
checks the C includes only freestanding headers, and the object's
undefined symbols (`nm -u`) are only the platform's. Then it links the
object with `tests/shared/platform.c`, a platform for the test machine,
and compares what the program does with what the interpreter does, as for
the hosted build. That platform also checks the runtime's side of the
contract: every block is freed or resized with the size and alignment
it was given with. `tests/freestanding.rs` checks what can't be built
for it, and that each error is where it should be.

## Design decisions worth noticing

- **A build flag, not a manifest field.** A package says what it needs,
  a build says where it runs. One program can be built for both, and
  tested on the hosted one. One that writes the platform builds for the
  freestanding target alone, but is still checked and tested as any
  other (step 71).
- **The platform in C, for now.** The five functions are a C interface,
  which a board's start-up code, written in C or assembly anyway,
  provides. Writing them in T needs a way to reach hardware
  (volatile memory accesses) and to call and be called by C: the next
  step.
- **One runtime, two layers,** rather than a second runtime for bare
  metal: the hosted target is a platform like any other, so what runs on
  a board is what the corpus tests on every commit.
- **Sizes given back to `free`.** It costs nothing, as the runtime
  knows them, and lets a platform's allocator be a few lines: a bump
  allocator, a free list per size.
- **No stack protector.** A freestanding object is compiled with
  `-fno-stack-protector`, as a protector needs a runtime to report to.
  Running out of stack is still no effect's (docs/33-c-backend.md, C5).

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| calling C, and being called by it (`extern fn`) | the platform's functions are enough to run | 40 ✅ |
| volatile accesses, memory-mapped devices | needs addresses, `unsafe`, and `extern` | 40 ✅ |
| the platform's functions written in T | needs the two above | 40 ✅ (write and halt) |
| 32-bit machines | the backend assumes 64-bit pointers and `usize` | — |
| interrupt handlers, `#[repr(C)]` | with FFI | — |

## Exercises

1. Write a platform for a machine you have: a `t_platform_write` that
   writes to a UART at a fixed address, a `t_platform_halt` that spins,
   and a bump allocator over a static array whose `free` does nothing.
   Why is that allocator's `free` allowed to do nothing?
2. Why does a freestanding `main` have to give back `()`, when a test
   (docs/31-tests.md), which has no std either, can give back a
   `Result`?
3. A static holding a `&'static str` becomes a pointer to a literal in
   its initializer. Why is that still a constant expression in C?
4. Why is the target a build flag but `libraries` a manifest field?
   What would go wrong the other way round?

## Reading

- ISO/IEC 9899:2011 (C11), §4 *Conformance*: hosted and freestanding
  implementations, and the headers a freestanding one must have.
- The GCC manual, *Options Controlling C Dialect* (`-ffreestanding`), and
  its note that freestanding code must still provide `memcpy`, `memmove`,
  `memset` and `memcmp`.
- The Rust Embedded Book, and the Rust reference on `#![no_std]`,
  `#[panic_handler]` and `#[global_allocator]`: the same split, made in
  the language.
- Philipp Oppermann, *Writing an OS in Rust* (os.phil-opp.com): a
  freestanding binary, step by step.
