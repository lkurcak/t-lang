# Step 33: a C backend

Code: `cgen.rs` (C from MIR), `cgen/shim.h` (the runtime every compiled
program starts with), `mono.rs` (`Shape`, `Executable::closure_body`),
`lib.rs` (`build_c`), `main.rs` (`t build`), `tests/compiled.rs` (the
differential tests).

Until now a T program only ran inside `t`, interpreted. `t build` makes
a native program of it, by way of C: it writes one C file and has a C
compiler (`$CC`, or `cc`) compile it.

```rust
#[derive(Discard)]
enum Shape { Circle(i64), Square(i64) }

fn area(s: &Shape) -> i64 with panic {
    match s {
        Shape::Circle(r) => 3 * *r * *r,
        Shape::Square(a) => *a * *a,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&area(&Shape::Circle(2)))?; // expect: 12
    out.line(&area(&Shape::Square(3)))?; // expect: 9
    Result::Ok(())
}
```

```text
$ t build shapes.t      # writes ./shapes; -o names it, --emit-c prints the C
$ ./shapes
12
9
```

`t run` stays what it was: the interpreter, which checks as it runs what
the checker promised (refinements, effects, leaks, `unsafe` blocks'
obligations). A compiled program trusts those promises and runs a
hundred times faster or more.

## One translation of what the interpreter runs

The backend design pass (B1) named the risk of a backend: a second
meaning of the language, which agrees with the interpreter's until it
doesn't. So the C backend has nothing of its own to decide. Its input is
exactly what the interpreter runs: the copies of each function's MIR that
monomorphisation made (step 32), with every drop and drop flag that drop
elaboration worked out, and the glue (step 32.1). Each MIR statement
becomes the C that does what the interpreter does for it, in the same
order. Here is `area`'s MIR, as `t mono` prints it, and its C, as
`t build --emit-c` prints it:

```text
bb0: {
    match _1 { &#0(_) => bb1, &#1(_) => bb2 };
}
bb1: {
    _2 = &((*_1) as #0).0;
    _4 = 3_i64;
    _3 = move _4 * copy (*_2);
    _0 = move _3 * copy (*_2);
    dead _2;
    goto bb3;
}
...
```

```text
static int64_t f_area(T_Shape* _1) {
    int64_t _0 = {0};
    int64_t* _2 = {0};
    ...
bb0:;
    if (((*_1).tag == 0)) goto bb1;
    if (((*_1).tag == 1)) goto bb2;
    t_unreachable();
bb1:;
    _2 = &(*_1).u.v0.f0;
    _4 = ((int64_t)INT64_C(3));
    _3 = t_mul_i64(_4, (*_2), 6);
    _0 = t_mul_i64(_3, (*_2), 7);
    goto bb3;
...
```

Every local is a C local, every block a label, every jump a `goto`. A
`match` tests its patterns in order; the checker made sure one matches,
and `t_unreachable()` stops the program if none does, rather than
telling the C compiler it can't happen. `dead`, `inspect`, `kept` and
`refine` are there for the borrow checker and the interpreter only, and
become nothing. `drop p` calls the drop glue of `p`'s type on it, and
`free p` frees a box.

## Values in C

How values are laid out is unspecified (decided in the backend design
pass): no T program can tell, as there's no `transmute` and no FFI. For
now:

| T | C |
|---|---|
| `i8`..`i64`, `isize` | `int8_t`..`int64_t`, `int64_t` |
| `u8`..`u64`, `usize` | `uint8_t`..`uint64_t`, `uint64_t` |
| `bool`, `char` | `bool`, `uint32_t` |
| a struct, a closure | a C struct of its fields, in order (a closure's are what it captured) |
| an enum | a `uint32_t` tag, and a union of a struct for each variant with fields |
| `&T`, `&mut T`, `Box<T>` | `T *` |
| `&[T]`, `&mut [T]`, `&str` | a struct of a pointer and a length |
| `[T; N]` | a struct of a C array, so that it can be assigned |
| `fn(A) -> R` | a C function pointer |
| `()`, `!`, what holds nothing | a struct of one byte, which nothing reads |

ISO C has no empty structs, nor arrays of no values, so a type that
holds nothing gets one byte (D1). Leaving those out altogether, and
packing `Option<&T>` into a pointer that may be null, are optimisations
for later, which the unspecified layout leaves room for. Pointers must be
64 bits (D4), as the interpreter's `usize` is: the C says so with a
`_Static_assert`, so a 32-bit target fails to compile instead of
computing something else.

## No undefined behaviour

C leaves much undefined: overflowing a signed integer, shifting by the
width of a type or more, dividing the least `i64` by -1, reading what
was never written, reading a union's member other than the one last
written, converting a pointer to an unaligned one, adding to a null
pointer. A compiler may assume none of those happen, so where one does,
anything can. The backend writes C in which none can (C2):

- **Integers** are computed by the runtime's helpers, in `int64_t` or
  `uint64_t`, after checking that the result fits: `t_mul_i64(a, b,
  site)` is `a * b`, or the panic `integer overflow`. Narrow types aren't
  computed in C's `int`, where `65535u16 * 65535u16` would overflow it.
  A conversion to a signed type that may not fit goes through `t_sx`,
  which works out the two's complement number from the bits.
- **Shifts** never shift by the width or more: `x << n` is `x * 2^n`,
  checked (lost bits are an overflow, as in step 12), and `x >> n` past
  the width is 0, or -1 for a negative `x`.
- **Every local starts zeroed** (`= {0}`), so no read is of what was
  never written, whatever a lowering bug might do. The C compiler drops
  the stores it can see are dead.
- **A union's member** is read only where the tag says it's there: the
  MIR reads a variant's field only after a `match` on it.
- **Pointers** point only to the type they're of, so there's no type
  punning for strict aliasing to break. An empty slice or buffer points
  to a real object, aligned for anything, rather than being null, so
  `ptr + 0` is defined.
- **Sizes** are checked before `malloc`: a `RawBuf` of more slots than
  fit in memory runs out of memory, rather than allocating too little.

What the checker proved can't overflow is computed without the check,
but still without undefined behaviour (C3, D4 of the backend design):
`t_wmul_i64(a, b)` multiplies modulo 2^64. If the solver were wrong, the
program would compute a wrong number, not anything at all. The C never
tells its compiler what the T checker proved (`__builtin_unreachable`,
`restrict`, `assume`), as a bug in the checker must not become one in
the compiled program.

The differential tests check this too: CI runs them again with every
program built with `-fsanitize=address,undefined`, which stops a program
at the first undefined behaviour, the first memory error, or, at its end,
the first leak.

## Panics, and where they're shown

A panic writes what the interpreter writes, to `stderr`, and the program
exits with 1, as `t run` does:

```text
runtime error: panicked: `unwrap` of `None`
 --> main.t:3:13
  |
3 |     let _ = x.unwrap();
  |             ^^^^^^^^^^
```

The location and the line of source are written into the C, one string
per place a panic can be shown. A panic in a library is shown where the
program called into it (step 31's `Error::blamed_caller`), which the
interpreter works out from its stack of calls. A compiled program has
none to look at, so it keeps the one thing the interpreter looks for:
`t_site`, the innermost call made from the program's own code. Each such
call saves it, sets it, and puts it back after (D2):

```text
{ uint32_t s_ = t_site; t_site = 3; _5 = f_Option_i64_unwrap(_6); t_site = s_; }
```

That's two stores per call from the program's own code, and none from a
library's. As a panic aborts, nothing is dropped on the way out, and
nothing unwinds.

## The runtime

`cgen/shim.h` starts every compiled program: the integer helpers, panics,
`malloc` and `free` (running out of memory is `alloc`'s abort, step 16),
and the intrinsics:

- `RawBuf<T>` is a pointer and a capacity; `grow` is `realloc`.
- `Arena<T>` is a growable array of pointers to its values, each in a
  box of its own, so a reference `alloc` gave stays put. Dropping it
  drops each value, in the order they were put in, by its box's glue, and
  then the array.
- `Message` (what `panic_with` writes) goes straight to `stderr`, as
  nothing can come between making one and aborting with it, so no buffer
  is needed, and none can fill up.
- The streams are C's: `fgetc`, `fputc`, with errors as `std::io::error`
  reads them, a broken pipe as one (`SIGPIPE` is ignored, as Rust does).

It needs a hosted C11 library: `stdio.h`, `stdlib.h`, `string.h`. A
freestanding target, with no C library, needs a `panic` sink and an
allocator of the program's own; that's for later.

## Why it's sound

- **C1, one meaning.** The C is a statement-by-statement translation of
  the MIR the interpreter runs. What's dropped, where, behind which flag,
  and in which order, was decided once, by drop elaboration, and both run
  that. What's left to agree on is what each statement does, which the
  differential tests compare on every program of the corpus.
- **C2, no undefined behaviour,** as above: each construct the C uses is
  defined for every value it can get, not just those the checker allows.
- **C3, proofs aren't trusted with more than a wrong number.** A proved
  operator wraps; an index into a slice is checked anyway (as it costs
  a compare, and a wrong proof there would be a memory error).
- **C4, what's left out.** The interpreter's checks of refinements,
  effects, leaks, and its `unsound(..)`s, are each a theorem of the
  checker about every program it accepts; the interpreter checks them
  on every run of the corpus, and the compiled program leaves them out.
  An `unsafe` block's obligations are the programmer's to keep: a
  compiled program that breaks one has undefined behaviour, which is what
  `unsafe` means. `t run` checks them.
- **C5, the stack.** Recursion that runs out of stack is stopped by the
  operating system's guard page, where the interpreter stops it at 10 000
  calls. No effect says a function may run out of stack (B7).

## Testing it

`tests/compiled.rs` runs every program of `tests/t`, `examples/` and the
docs both ways, with the same input, and compares what each writes to
its output and to `stderr`, panic messages and locations included, and
how it ends. It leaves out programs that end where only the interpreter
can say: an `unsafe` block breaking its obligations, and running out of
the interpreter's own memory or stack, which are smaller than a
machine's. Mutation checks (leaving out each `free`, the overflow check of
`+`, or the saving of `t_site`) each make it fail.

## Design decisions worth noticing

- **C, not LLVM IR or a code generator of our own** (backend design
  pass): every target has a C compiler, the output is readable, and the
  optimiser is free. What C costs is its undefined behaviour, which the
  backend has to stay clear of by construction.
- **Nothing decided twice.** Drop elaboration and glue moved out of the
  interpreter in step 32.1, so that this translation has no decisions of
  its own about what drops, or how `==` compares.
- **Zeroed locals and checked indices** cost a little speed for C2 and
  C3. They're the first things to take out if a profile ever says so,
  each with an argument for why it's safe.
- **`#line` directives** in what `t build` compiles make a debugger, and
  the C compiler's messages, point at the T source. `--emit-c` leaves them
  out, to keep the C readable.

## Deliberately missing (coming later)

- Erasing types that hold nothing, niches (`Option<&T>` as a pointer),
  field reordering: optimisations the unspecified layout allows.
- `t test` compiled: tests still run interpreted.
- Freestanding targets (no C library), 32-bit targets, Windows.
- FFI and `#[repr(C)]`, which would make layout observable.
- An effect for running out of stack (B7).

## Exercises

1. Why can't `t_add_i32` compute `a + b` in C's `int32_t` and check after?
2. `x >> 64` of an `i64` is defined in T. Write the C that computes it
   without undefined behaviour, for `x` negative and positive.
3. A library function calls a closure from the program, which calls
   `unwrap` on `None`. Where is the panic shown, and what does `t_site`
   hold at each step?
4. Why does an `Arena`'s drop glue live in the runtime and not in MIR?
5. Which of the interpreter's checks would catch a bug in drop
   elaboration, and which compiled check would catch it instead?

## Reading

- John Regehr, *A Guide to Undefined Behavior in C and C++* (2010), and
  the CERT C rules INT32-C and INT34-C: checking arithmetic before it
  overflows.
- Xuejun Yang et al., *Finding and Understanding Bugs in C Compilers*
  (PLDI 2011): Csmith, differential testing of compilers.
- William McKeeman, *Differential Testing for Software* (1998).
- The Chicken Scheme and Nim compilers, which also compile through C.
