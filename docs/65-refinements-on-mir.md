# Step 65: refinements on MIR

Code: `src/check/refine/mir.rs`, the walk over MIR; `Checker::refine`,
`Checker::compare` and `Refiner` in `src/check/refine.rs`; `Notes` in
`src/mir.rs`, which `src/borrowck/build.rs` fills as it lowers a body; and
`tests/refine_mir.rs`.

The refinement checker walks each body's AST, expression by expression,
and most of its holes have been ways of getting at a place it had no case
for (docs/63-what-is-declared.md). MIR has one grammar of places, a local
and then fields, variants' fields and `*`s, and by the time it's built every
desugaring (`for`, `?`, `if let`, `let else`, indexing, autoref) has been
done once, the way it runs. Step 64 moved the borrow checker, which builds
the MIR, before refinements. This step adds a second refinement checker,
on MIR, with the same rules, beside the walk over the AST, and checks that
they agree. Step 66 makes it the only one.

So nothing changes for a program. The walk over the AST still decides
what's accepted and which operators can't panic. Set `T_LANG_REFINE=mir`
and the walk over MIR decides instead; set `T_LANG_REFINE=compare` and both
run on every body, the AST's decides, and `t` says where the MIR's found
anything else, and exits with status 3.

```rust
fn small(n: u8) -> Option<{ v: u8 | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn sum(a: [u8; 5]) -> Option<u32> {
    let mut total: u32 = 0;
    let mut i: usize = 0;
    while i < 5 invariant total <= 3 * i {
        let v = small(a[i])?;
        total = total + v as u32;
        i = i + 1;
    }
    Some(total)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    match sum([1, 3, 0, 2, 2]) {
        Some(t) => out.line(&t)?, // expect: 8
        None => out.line(&0u32)?,
    }
    out.line(&sum([1, 9, 0, 0, 0]).is_some())?; // expect: false
    Ok(())
}
```

Both walks prove the same here: that `a[i]` is in range, from the loop's
condition; that `total + v as u32` can't overflow, from the invariant and
what `small` returns, through the `?`, a `match` on MIR as it is on the AST
once it's desugared; and the invariant again at the end of the body. So
`sum` has no `panic` in its effects, by either.

## How the walk gets around MIR

The walk over the AST has two halves. One is what's known and how it's
proved: values, places and the objects they are, facts, measures, what
type arguments' refinements say (`targs`), the logic. The other gets
around a body and finds places: a case for each kind of expression and
pattern. The walk over MIR keeps the first half, the same functions, and
replaces the second.

- **Places and values.** A variable or a parameter is the place of its
  binding, as on the AST, so a predicate names it the same way, by its
  binding's id. A temporary, or the function's result, is no place but a
  value (`Slot`), as an expression's value is on the AST: MIR's
  temporaries are what the AST's walk carries from an expression to the
  one around it. A borrow of a temporary, a field of one or a `*` of one
  is worked out from its value, as the AST's walk does `&f()` or `f().x`.
- **Blocks.** The blocks are walked in reverse postorder, so each comes
  after every block that leads to it but along an edge back to a loop's
  head, and of two branches, the first comes first, as on the AST. Along
  an edge, what's known is what the block knew, and the condition of a
  `Branch`, or for a `Match`, that the value matches the arm's pattern and
  none before it (`pat_formula`). Where paths meet, what's known is what
  the AST's `join` says of branches: the facts all of them share, from
  where they parted, and that one of them was taken, with each place and
  temporary whichever value that path gave it.
- **Loops.** At a loop's head (`Body::heads`), from the paths into it from
  outside: its invariant holds; then each place it changes, as any
  assignment or `&mut` in it says, is forgotten, but for the measures its
  frame keeps; and its invariant is assumed. Along each edge back to the
  head, from the end of its body or a `continue`, its invariant must hold
  again. Its exits are just paths that meet after it. This is the Hoare
  rule of docs/54-loop-invariants.md, on a graph.
- **Calls.** A `&mut` that's an operand of a call is lent to the call: its
  place is forgotten once the call returns, when what the callee ensures
  of it is learned. Any other `&mut` forgets its place where it's made.
  That's what the AST's walk does too, where a `&mut` is written in a
  call's arguments.
- **Closures.** A closure is walked from what's known where it's made, as
  on the AST, but its body is its own MIR body, whose first parameter
  holds what it captured: a field of it is the place of the variable it
  captured. One given straight to a function is walked at the call, after
  the call's other arguments, counting on what they say of its parameters
  (`closure_trees`), as the AST's walk does a closure literal in a call.
- **Patterns.** A binding reads the place the pattern took apart, through
  `*`s, fields and variants' fields; a variant's field is what's known of
  that variant of the enum's value (its parts), as on the AST. While a
  pattern's variables are bound, the value matched is the one the arm's
  condition is about (`State::pinned`), not the place read again: a place a
  pattern binds by `&mut` gives a new value each time it's read.

MIR doesn't keep everything the walk needs: which binding each local is,
where an argument or a value being assigned is in the source, the text of
a place written `&mut`, which `Return` is the end of the body, each loop's
blocks and where its `continue`s are, and the node of the `-` of a
negated literal, which MIR folds into a constant. The lowering writes those
down as it goes (`Notes`); nothing that runs reads them.

## Why it's sound

The walk over MIR is used only when asked for, so no program's meaning
changes in this step. But step 66 will make it the one, so it has to be
sound by itself.

> **Lemma M1 (facts hold along every path).** At the start of each block,
> every fact the walk has holds of the values the program has there, on
> every path from the body's start that gets there. By induction along
> reverse postorder: the entry block's facts are the parameters'
> refinements; a block's statements add only what the AST's walk would of
> what they were lowered from; an edge adds its condition, which holds
> along it; and where paths meet, the facts kept are those on every path
> (a prefix they share, from where they parted), plus the disjunction of
> each path's own, one of which holds as one path was taken. A loop's
> head is reached from outside, and along its back edges: the facts from
> outside hold of what the loop doesn't change, which is all that's kept
> of what it does (with its frame, docs/52-loop-frames.md), and its
> invariant, assumed, was proved along each edge into the head.

> **Lemma M2 (every place written is seen).** A place changes only by a
> statement that assigns it or a part of it, by a `&mut` that may be
> written through, or behind the walk's back, by a closure or a pattern's
> `&mut`. The first two are all statements of MIR's grammar that write:
> an `Assign`, whose rvalue is one of seven kinds, each a `match` arm with
> no catch-all, and a `Ref`. The rest are what the walk calls unstable,
> found once before it starts, by the same rule as on the AST. So, as
> lemma W of docs/63-what-is-declared.md says of the AST, there is no way
> of writing a place the walk has no case for; on MIR, that's a property
> of the grammar rather than of how many cases were written.

> **Lemma M3 (the same rules).** Each statement is checked by the function
> the AST's walk uses for what it was lowered from: an assignment by the
> same checks of the variable's refinement and of what's declared there, a
> struct's literal by its fields' refinements, a call by its parameters'
> refinements and postconditions, a loop by `loop_start`, a result by
> `result`. So what's needed, and what's assumed, is what the AST's walk
> needs and assumes, and the soundness arguments of steps 8 to 63 carry
> over unchanged.

## Parity, checked

`tests/refine_mir.rs` checks every program in `tests/t` and `examples`
(2,281) and the libraries with both walks over every body, and they must
agree: the same errors, with the same messages, at the same places, with
the same notes; the same operators proved not to panic; the same consts'
values; and the walk over MIR may not take more than twice the work. They
do, on all of them, but for three where a failed proof is explained with
the same facts in another order (MIR works out `*x as u64` into a
temporary before it reads `sum`, so the explanation says `(*x) + sum`),
which `tests/refine_mir.allow` lists. The programs using `t` (t-libs,
tcalc, t-nonograms, t-maze, t-chess) agree too, with one such explanation
in t-libs. And the whole test suite passes with `T_LANG_REFINE=mir`.

A harness that always agrees could be one that never looks. So each of
the walk's rules was switched off in turn (an `&mut` not forgetting its
place, a loop's head not forgetting what the loop changes, a join keeping
one path's facts, a pattern's arm not excluding the ones before it, and
so on), and each time the harness found the walks differ.

## Decisions

- **A switch in the environment, not a flag.** `T_LANG_REFINE` is for
  developing `t` and goes in step 66; a flag would have had to be passed
  through every command and its help, to be taken out again.
- **The same, or better for a reason.** The walks must find the same,
  but where the one on MIR proves more for a reason the AST's can't see,
  it may be listed with that reason. None is: on MIR, an or-pattern's
  variables have the values of the alternative that matched, where the
  AST's walk knows nothing of them, but no program in the tests needs it.
  Such gains come with step 66, each with its argument.
- **Today's rule for what's unstable.** A place a closure captures, or a
  pattern binds by `&mut`, is still unknown for the whole function, as
  on the AST. The borrow checker knows when each borrow ends, and could
  say when the place is known again, but that's a rule of its own to
  argue, not a port.

## What it doesn't do

- It doesn't decide anything yet: step 66 makes it the refinement
  checker, and removes the walk over the AST.
- It finds places on MIR, but checks only those the AST's walk did.
  Variants' fields as places a `&mut` can be bound to, which the AST's
  walk refuses (docs/62-t-chess-findings.md), are step 66's.
- Explanations of failed proofs can list terms in another order than the
  AST's walk does.

## Exercises

1. Run `T_LANG_REFINE=compare t check` on a program of yours. Then change
   `join_states` in `src/check/refine/mir.rs` to keep only the first
   path's facts, and run it again. Which programs in `tests/t` would that
   make the walk over MIR accept that it shouldn't?
2. Why does the walk at a loop's head forget what the loop changes before
   it assumes the invariant, but prove the invariant before it forgets?
3. A `&mut` given straight to a call is forgotten after the call, any
   other where it's made. Write a program where forgetting it where it's
   made, for one given to a call, would fail a proof that holds.
4. Why is a temporary a value and not a place? What would the walk need to
   know of a temporary that's borrowed `&mut` and given to a call?

## Reading

- Nico Lehmann, Adam Geller, Niki Vazou, Ranjit Jhala, *Flux: Liquid
  Types for Rust* (PLDI 2023): refinement types checked on MIR, joins at
  basic blocks, invariant `&mut`.
- Cormac Flanagan, James B. Saxe, *Avoiding Exponential Explosion:
  Generating Compact Verification Conditions* (POPL 2001): what's known
  where paths meet, as a disjunction over the paths, without copying what
  they share.
- Alfred Aho, Monica Lam, Ravi Sethi, Jeffrey Ullman, *Compilers:
  Principles, Techniques, and Tools*, chapter 9: reverse postorder, back
  edges and natural loops of reducible flow graphs.
