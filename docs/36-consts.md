# Step 36: consts

Code: `parser.rs` (`Parser::const_decl`), `check/consts.rs` (plain data,
the initializer's effects, consts in patterns and as lengths),
`check/paths.rs` (a use is the const's value), `check/refine.rs`
(`Known`, `Walk::const_item`), `check/infer.rs` (`body_groups`),
`consts.rs` (working values out), `mono.rs` (`rooted`, `Mono::lengths`),
`interp.rs` (`evaluate`, `Interp::bits`), `types.rs` (`Type::Const`), and
`traits.rs` (`unify`).

A const item names one value, fixed when the program is compiled:

```rust
const SIZE: usize = 4;
const LAST: usize = SIZE - 1;
const GREETING: &str = "hello";

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut buf = [0u8; SIZE];
    buf[LAST] = 7;
    out.line(&buf[LAST])?; // expect: 7
    out.line(&GREETING)?;  // expect: hello
    Result::Ok(())
}
```

Its type is always written, as a function's is. Its name is in upper case
(`non_upper_case_globals`), as a const parameter's is: the case was kept
for this since step 6 (docs/06). It's private unless it says `pub` or
`export`, like any item, and it's declared in a module, not in a block.

## Worked out while compiling

The initializer is any expression, calls and loops included. Once the
program checks, the interpreter runs it, and the const's value is what it
gives:

```rust
fn squares() -> [u32; 4] {
    let mut a = [0u32; 4];
    let mut i: usize = 0;
    while i < 4 {
        let n = i as u32;
        a[i] = n * n;
        i = i + 1;
    }
    a
}

const SQUARES: [u32; 4] = squares();

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&SQUARES[3])?; // expect: 9
    Result::Ok(())
}
```

What it runs, it runs once. The const's body is then replaced by one that
builds the value, `_0 = [0_u32, 1_u32, 4_u32, 9_u32]` in `t mono`'s
output, so neither the interpreter nor the C program runs `squares` again.

The interpreter can only do at compile time what doesn't need the world.
So the initializer may *panic*, and do nothing else (C1):

- no `alloc`: there's no heap to keep, as a value is made again at each
  use;
- no `unsafe`: what an `unsafe` block promises would be promised to no
  one;
- no I/O, which already needs the `Os`, and no const can have one.

```rust
use alloc::boxed::Box;

fn size() -> usize with alloc {
    let b = Box::new(3usize);
    *b
}

const N: usize = size();
// error: a const's initializer can't have the effect `alloc`

fn main() {}
```

A panic while it runs is an error while compiling, at the const, with
where it panicked:

```rust
const BIG: u8 = 200;
const SUM: u8 = BIG + 100;
// error: evaluating `SUM` panicked: integer overflow

fn main() {
    let _ = SUM;
}
```

So a use of a const can't panic, and has no effects at all: `fn f() -> u8
with {} { SUM }` is fine wherever `SUM` itself was. Nothing says a loop
ends (there's no divergence effect yet, docs/07), so the interpreter has
a budget, a million basic blocks, and running out of it is an error too.

## One value on every machine

Checking doesn't depend on the machine: a `usize` is 32 or 64 bits, and
proofs hold for both (docs/11). A const has to fit that, so its
initializer runs twice, on a 64-bit machine and on a 32-bit one, and the
two must agree (C2):

```rust
const MAX: usize = core::num::greatest::<usize>();
// error: `MAX` is 18446744073709551615 on a 64-bit machine but 4294967295 on a 32-bit one

fn main() {
    let _ = MAX;
}
```

`65536 * 65536` as a `usize` panics on the 32-bit run, and is an error for
the same reason. What checked arithmetic gives doesn't depend on the width
unless it overflows, so only wrapping, bitwise operations and the bounds
of `usize` and `isize` can tell the machines apart, and the two runs catch
those.

## Plain data

A const's type is *plain data*: integers, `bool`, `char`, `()`, `&str`,
`&[u8]`, and arrays, structs and enums of those (C4). Not `&mut`, not
other references, not functions, nothing intrinsic such as a `Box`:

```rust
const R: &mut u8 = &mut 1;
// error: a const can't be of type `&mut u8`

fn main() {}
```

That's what the interpreter can hand back as a tree and a use can build
again. A `&'static` in a const can only point into a literal: there are no
statics, and nothing a body makes lives that long. So a string or bytes
cover every reference a const can have. A reference left out in a const's
type is `'static`, and `&str` means `&'static str`.

A const's type needn't be `Copy`. Each use is a value of its own, as if a
function of no parameters made it:

```rust
#[derive(Discard)]
struct Point { x: i64, y: i64 }

const ORIGIN: Point = Point { x: 0, y: 0 };

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = ORIGIN;
    let b = ORIGIN;          // another one, not a move of `a`
    out.line(&(a.x + b.y))?; // expect: 0
    Result::Ok(())
}
```

A const is a value, not a place: `ORIGIN = ..` is an error, and `&ORIGIN`
borrows a temporary, as `&f()` would.

## As a length

A const can be an array's length, or the argument of a const parameter,
written as a name or a path:

```rust
mod sizes {
    pub const BLOCK: usize = 8;
}

#[derive(Discard)]
struct Block { bytes: [u8; sizes::BLOCK] }

fn zeros<const N: usize>() -> [u8; N] { [0; N] }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let b = Block { bytes: [1; sizes::BLOCK] };
    out.line(&b.bytes.len())?;                 // expect: 8
    out.line(&zeros::<sizes::BLOCK>().len())?; // expect: 8
    Result::Ok(())
}
```

As a length it's equal only to itself, as a const parameter is: `[u8;
BLOCK]` isn't `[u8; 8]` (C6).

```rust
const N: usize = 3;

fn main() {
    let _a: [u8; N] = [1, 2, 3];
    // error: expected `[u8; N]`, found `[u8; 3]`
}
```

That keeps what a type is independent of what an initializer works out
to. Types are checked before any const has a value, and an initializer
can call any function, so making `N` equal to `3` would need its value in
the middle of checking. The restriction is one that can be lifted later,
with one care: two impls, for `[u8; N]` and for `[u8; 3]`, would overlap
then. So coherence treats a const length as any length already, and they
overlap now:

```rust
const N: usize = 3;

trait Name {
    fn name(&self) -> u8;
}
impl Name for [u8; N] {
    fn name(&self) -> u8 { 1 }
}
impl Name for [u8; 3] {
    fn name(&self) -> u8 { 2 }
}
// error: overlaps

fn main() {}
```

Monomorphisation then replaces each const length by its value, the same
everywhere (C2). So lemma A1 of docs/22 holds: an array has as many
elements as its type says.

## What the logic knows

The refinement logic knows a const's value if it can tell it from the
initializer: a literal, or arithmetic on literals and on consts it knows
(C5). That's why `buf[LAST]` above needs no proof from the program:
`LAST` is `3` and `SIZE` is `4`, so `3 < 4`. Of a call, it knows what the
function's type says, as of any call, and of what it doesn't know, it
knows what the const's own type says:

```rust
fn far() -> { n: u32 | n > 100 } { 1000 }

const LIMIT: { n: u32 | n > 100 } = far();

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(LIMIT - 100))?; // expect: 900
    Result::Ok(())
}
```

`LIMIT - 100` can't underflow, as `LIMIT > 100`, but nothing proves `a[LIMIT
as usize]` in bounds for an array of 200. A const of an integer type or
`bool` can be named in a refinement too: `{ i: usize | i < SIZE }`.

The interpreter checks the logic: if it took a const to be `c`, the value
must be `c`. Where the logic has it is where the const's body was
checked, so bodies are checked in an order with each const as early as
what it calls lets it (`body_groups`). That only adds facts.

## Depending on itself

What a const needs is worked out first: another const's value, by a use
or as a length, anywhere in what its initializer runs. A const that needs
itself has no first (C7):

```rust
const A: u8 = f();

fn f() -> u8 { A }
// error: the value of const `A` depends on itself

fn main() {
    let _ = A;
}
```

The dependency is found by monomorphising the const alone
(`mono::rooted`), with only what it reaches, and what that names whose
value isn't known yet. So it's what the code could run, not what it runs:
a branch never taken still counts.

## Why it's sound

- **C1, evaluable.** The initializer's effects are at most `panic`, so it
  touches no heap, no raw memory and nothing outside the program. What
  the interpreter computes is what the program would. A panic is a
  compile error, so a use never panics and has no effects.
- **C2, one value.** Checked operators give their exact result or panic.
  Only the width of `usize`, which is 32 or 64 bits, can make a result
  differ, and both are run, so the value is the same on every machine,
  and checking stays independent of it.
- **C3, a fresh copy.** A use is a call of a function of no parameters
  with a fixed result: linearity, invariants and privacy are as for any
  call. Privacy is checked in the initializer, as in any body.
- **C4, plain data.** A plain value has no address in it but a literal's,
  so a use can build it again.
- **C5, the logic.** A fact about a const comes from what the refinement
  checker proved of its initializer, so it's as sound as that is. A fact
  missing because of the order costs only completeness.
- **C6, lengths.** Equal only to itself is a restriction; overlapping as
  any length keeps coherence if it's later lifted.
- **C7, order.** A const that depends on itself is rejected, so the rest
  form a DAG, and each is worked out after what it needs.

## Design decisions worth noticing

- **No `const fn`.** Rust marks the functions a const can call. Here the
  effects already say it: what has at most `panic` can run while
  compiling. Effects did the work a second function colour would.
- **Twice, not at the target's width.** The values could have been the
  target's, but then a program would check differently per machine. Two
  runs keep checking independent of the machine, and only constants that
  really are different get rejected.
- **Opaque lengths.** Rust normalises `N` to its value. That needs
  evaluation interleaved with type checking, which is a query system, not
  phases (retrospective 3's H5).
- **The logic's facts are its own.** The evaluated value isn't fed back
  into proofs, as it isn't known while bodies are checked. What the logic
  proves of the initializer is, and the interpreter checks the two agree.
- **Not in patterns.** In Rust, `N` in a pattern compares with the const
  if one is in scope, and binds otherwise, so a typo silently binds. Here
  a pattern that names a const in scope is an error, which keeps the
  question open.

## Deliberately missing (coming later)

- Consts in patterns, as above.
- Associated consts, in traits and impls, and consts in blocks.
- `&CONST` as a `&'static`: needs statics, and frozenness (which step 37
  added: a const has no `Cell` in it).
- Generic arithmetic on lengths, `[T; N + 1]`, which the logic can decide
  (linear arithmetic).
- Statics, `static` items, which are places, not values: with `Cell`, a
  `global` effect and `Sync` (steps 37 and 38).
- Library consts (`u8::MAX`): core has none yet.

## Exercises

1. Why must `65536 * 65536` as a `usize` be rejected, even though every
   machine that runs the program could be 64-bit? What would checking
   lose?
2. Write a const whose value the logic can't tell, but which a refinement
   on its type lets a proof use anyway.
3. Find a program that would type-check if `[u8; N]` were `[u8; 3]`, and
   whose meaning would then depend on `N`'s value.
4. A const's initializer is run with a budget. What would a divergence
   effect (docs/07) let the checker say instead?
5. `const A: u8 = if false { B } else { 1 };` with `const B: u8 = A;` is
   rejected. Why is that the right answer for a dependency that never
   runs?

## Reading

- The Rust Reference, *Constant evaluation* and *Constant items*: what
  Rust allows in a `const fn`, and how its consts are inlined at each use.
- Ralf Jung's posts on Miri and const evaluation: why compile-time
  evaluation must not do what can't be undone, and how an interpreter
  checks for undefined behaviour while doing it.
- Zig's `comptime`: evaluation at compile time as the same language, and
  what it costs to interleave it with type checking.
