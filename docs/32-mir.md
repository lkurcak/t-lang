# Step 32: one MIR, checked and run

Code: `mir.rs` (the MIR, its printer), `borrowck/build.rs` (lowering, now
with what running needs), `borrowck.rs` (`check` and `closure_body` keep
each body's MIR, `Info::mir`), `mono.rs` (`Cx::body`, `Cx::closure`,
`Cx::fn_const`, `Executable::render_only`), `interp.rs` (`Interp::run`,
`rvalue`, `write`, `dead`, `matches`, `unsafe_op`), `lift.rs`.

Until now a function body was lowered twice. The borrow checker lowered
it to MIR (docs/05-ownership.md) and checked that; the interpreter walked
the AST, after `lift` wrote out what the borrow checker had decided
(which places move, which patterns use up what they take apart). The two
had to agree on everything memory depends on: the order arguments are
worked out in, when a borrow passed to a call is taken, when each
temporary goes away, what `return` drops on its way out. A comment in
`build.rs` said so, and the test corpus checked it, program by program.
Nothing made it true.

Now there's one lowering. The borrow checker builds each function's MIR,
checks it, and keeps it. Monomorphisation copies the MIR, not the AST, for
each list of type arguments. The interpreter runs the copies. What the
borrow checker proved is about the code that runs, because it *is* the
code that runs.

```rust
use alloc::boxed::Box;

fn pick(b: bool, x: Box<i64>, y: Box<i64>) -> Box<i64> {
    if b { x } else { y }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&*pick(true, Box::new(1), Box::new(2)))?; // expect: 1
    Result::Ok(())
}
```

`t mono` prints the copies:

```text
fn pick(b: bool, x: alloc::boxed::Box<i64>, y: alloc::boxed::Box<i64>) -> alloc::boxed::Box<i64> {
    let _0: Box<i64>;
    let _1: bool; // b
    let _2: Box<i64>; // x
    let _3: Box<i64>; // y
    let _4: bool;
    let _5: Box<i64>;
    let _6: Box<i64>;
    bb0: {
        _4 = copy _1;
        if copy _4 { bb1 } else { bb2 };
    }
    bb1: {
        _5 = move _2;
        _0 = move _5;
        dead _5;
        goto bb3;
    }
    bb2: {
        _6 = move _3;
        _0 = move _6;
        dead _6;
        goto bb3;
    }
    bb3: {
        dead _4;
        dead _3;
        dead _2;
        dead _1;
        return;
    }
}
```

Each local `_n` has a type; `_0` is the result, the parameters come next.
A block is a run of statements ending in a jump. `copy` reads a place,
`move` takes the value out of it, and `dead _n` is where `_n` goes out of
scope: what's still in it is dropped, and its memory freed.

## What running needs that checking didn't

The borrow checker's MIR left out whatever doesn't matter to borrowing.
It needed some of it back to run:

- **Constants' values.** A literal was just "a constant" before; now it's
  `1_i64`, `"text"`, or a function at its type arguments, `id::<T>`,
  which monomorphisation resolves (`Const`).
- **Which operator it was.** Each `+`, `-`, `as` keeps its node, so the
  interpreter can tell whether the refinement checker proved it can't
  panic (`Info::proved`), as before.
- **What an aggregate makes.** A struct, a variant, a box, an array, a
  closure, with its type (`Agg`).
- **Which arm.** A `match` used to end in "go to one of these blocks",
  with the patterns' reads before it, for the borrow checker; which arm
  was left out. Now the terminator carries each arm's pattern, by index,
  not name (`Term::Match`, `Pat`). The reads stay where they were, so the
  borrow checker sees what it saw.
- **What only the interpreter checks.** A `let` with a refinement ends in
  `refine _n { v | pred }`; an `unsafe` block's span is kept, for the check
  that an unsafe operation is inside one; and a pattern that took apart a
  value without using it up says `kept`, so the interpreter can check the
  value really is `Discard` at the type it turned out to have.

All of it is additive. The borrow checker ignores the new parts, its
graph has the same blocks, statements and edges, and its messages are
the same, as the 1100-odd error tests show.

## Monomorphising MIR

A generic function's MIR is at its own type parameters. A copy
substitutes the type arguments into every local's type, and every call of
something generic, or of a trait's method, becomes a call of the copy it
needs (`Cx::fn_const`, by the same `target` that resolved the AST's calls).
A closure's body is copied with the function it's in, as its types are
that function's: `main::{closure#7}`. A comparison of what isn't an
integer, `a < b`, becomes the call it stands for, `Ord::lt(&a, &b)`, in
temporaries of its own: monomorphisation may add statements, since it
runs after the borrow checker is done.

What a copy moves is what the generic body moved: the borrow checker
decided by the types it knew, so a value of a type parameter `T` moves
even where `T` is `i64`. The interpreter checks that against the copy's
types (a copy of what isn't `Copy` is a checker bug), as before; it used to
get the decision from `lift`, which no longer rewrites bodies. Lifting is
only of declarations now, and a lifted function's body is empty.

## Running MIR

The interpreter runs a body a block at a time. A local gets a slot when
it's first written and loses it at its `dead`, which drops what's left in
it: all of it, part of it if something was moved out, or nothing. An
assignment drops what it replaces. That's the same dynamic rule the AST
interpreter had, so programs drop exactly what and when they did, and the
corpus prints exactly what it printed. Two checks are new:

- a function that returns with a local still in a slot has a lowering
  bug: something went out of scope without its `dead`;
- control reaching `unreachable`, after a call of a function returning
  `!`, would be one too.

## Why it's sound

- **S1, one lowering.** The borrow checker's guarantees are about the MIR
  it checked. The interpreter runs that MIR, up to monomorphisation. So
  "no use after move, no dangling reference" is a claim about the run, not
  about a second translation of the source that had to agree with the
  first.
- **S2, monomorphisation changes no control flow.** A copy has the
  generic body's blocks, statements and moves; only types and callees
  change, and those are what the checker checked every instantiation
  against: what a copy calls is what the bounds promised (the argument of
  docs/04-generics.md, unchanged). The `Ord` rewrite adds borrows of
  places that are only read, inside one statement.
- **S3, generic meaning wins.** Moves are decided once, on the generic
  body, so every copy moves and drops the same places
  (the bug of the generic move vs a copy at `i64`, fixed in 0.14.0, can't
  come back: there's no second decision to disagree).
- **S4, the oracles check the lowering.** Dangling references, double
  frees, use after move, leaks, effects per frame, unsafe operations, and
  now locals outliving their function and `kept` values that aren't
  `Discard`, are checked as the MIR runs: a lowering bug shows up as one
  of those, not as a wrong answer.

## Design decisions worth noticing

- **The borrow checker's MIR, not a new one.** The alternative was a new
  lowering after monomorphisation, simpler to write next to the
  interpreter, but a second translation again. One MIR is rustc's design
  too: borrow-checked, then optimised and code-generated.
- **Drops are still dynamic.** `dead _3` in `pick` drops `y` when `b` is
  true and nothing when it's false; the interpreter looks at the slot to
  know. A compiled program can't look: step 32.1 works out, from the
  moves, which drops always happen, which never do, and which need a
  *drop flag*, a `bool` set where the value moves, and the interpreter's
  look at the slot becomes the check that the flag is right.
- **`t mono` prints MIR.** It used to print T, which could be read back
  (a test did). MIR is what runs, so it's what's worth seeing; the cost is
  that the output is longer, and isn't T.

## Deliberately missing (coming later)

- Static drop elaboration, drop flags, and drop and `==` code ("glue") as
  MIR functions of their own (step 32.1).
- A C backend translating the MIR (step 33).
- Optimisations on MIR: none are needed to run it, and each would need its
  own argument.

## Exercises

1. In `pick`, which of `dead _2` and `dead _3` drop something when `b` is
   true? Write down where a drop flag would be set and read.
2. `f(Box::new(1), return 2)`: lower it by hand, and find the `dead` that
   frees the box.
3. Find the statement in `generics.mono` (in `tests/t`) where a value of a
   type parameter moves although the copy's type is `Copy`. Why must it
   move?
4. Why may monomorphisation add the borrows of `Ord::lt(&a, &b)` without
   borrow checking them again?
5. Make the interpreter skip `dead` and run the tests: which oracle
   catches it first, and for which programs?

## Reading

- RFC 1211, *MIR* (Rust), and Niko Matsakis, *Introducing MIR* (2016): why
  rustc moved borrow checking and code generation onto one graph.
- RFC 2094, *Non-lexical lifetimes*: borrow checking on MIR.
- The rustc dev guide, *Drop elaboration*: what step 32.1 does.
- Xavier Leroy, *Formal verification of a realistic compiler* (CACM,
  2009): CompCert, where each pass is proved to preserve meaning, the
  stronger version of "one lowering, checked".
