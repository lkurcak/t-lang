# Step 61: proofs that don't blow up, and what step 60 missed

Code: `valid`, `refuted`, `impossible`, `eliminate` and `nnf` in
`src/check/refine/logic.rs`; `Cand::all`, `Walk::join_trees`,
`Walk::join_values`, `Walk::knowledge` and `Walk::place_read` in
`src/check/refine/targs.rs`; `Walk::post`, `Walk::fields_of`,
`Walk::need` and `Walk::no_mut_borrow` in `src/check/refine.rs`;
`t_lang::check::work` and `tests/proof_work.rs`; `Iterator::skip` in
`core/iter.t`; `saturating_add` and `saturating_sub` in `core/num.t`.

A maze game (t-maze) was the first program written on step 60's
refinements in type arguments. It found two ways to make `t check` take
minutes, and five things step 60 should have proved but didn't. This step
is about both.

## Cases a proof doesn't need

The checker proves a goal by refuting its negation together with what's
known: a conjunction, except that some of what's known is a disjunction
(`x != y` is `x < y || x > y`; a refinement known only where a branch was
taken is `!took || p`). It splits on each disjunction, one case for each
way it can go, and refutes each case by Fourier–Motzkin elimination
(docs/08). Twenty disjunctions are a million cases.

The game's search had a struct with fields refined by `board.width() *
board.height()`, a loop that could `break`, and a store after. Each read of
`self` said the fields' refinements again, each with a disjunction in it,
and the loop's exit said all it knew where it broke. Most of that had
nothing to do with what was being proved: `self.found = Some(found)` needs
only `found.squares == width * height`, which one fact says. But each
disjunction doubled the cases, and each field doubled them again. With
0.76.0, the program below took 5 seconds to check with one field `gN`, 21
with two, 94 with three (`tests/proof_work.rs` writes it with any number):

```text
struct Game {
    board: Board,
    g0: { g: Graph | g.count() == board.width() * board.height() },
    you: { v: usize | v < board.width() * board.height() },
    found: Option<{ f: Found | f.squares == board.width() * board.height() }>,
}
...
    while k <= 60 {
        let lit = look(&self.board, Some(&found), upto);
        if ready(k)? {
            go_on = false;
            break;
        }
        k = k + 1;
    }
    self.found = Some(found);
```

Now it takes a quarter of a second, whatever the number of fields, for
three reasons.

**Said once.** The same fact said twice is one fact, and a disjunction
said twice is split once. Atoms without variables are `true` or `false`
where they're made: `0 == 0 || x < 3` is `true`, not a disjunction, and a
case that's `x < y && -1 == 0` is `false`. That was the other blow-up, an
enum of refined pairs in a list (`List<Wall<({ x: usize | x + 1 < width },
..), ..>>`), whose elements' disjunction over the variants had such
constants in each case: 512 cases, the solver's limit, and a failed proof,
or two minutes with more facts in scope.

**Backjumping.** Each case remembers which of the splits above it its
atoms come from. When a case is refuted, Fourier–Motzkin knows which
atoms it used, and so which splits the refutation depends on. If it
doesn't depend on the one just made, it refutes every other case of that
split the same way: those are skipped. A disjunction the proof doesn't need
costs one case, not a doubling, however many there are and whatever order
they're split in. This is dependency-directed backtracking, as SAT solvers
and description-logic tableaux do it.

**Flat carried refinements.** Step 60 keeps what's said of the values at
each position of each object (`List<{ v: usize | v < n }>`'s elements). A
value read from an object took what was said of the object's, and a
call's result what was said of its arguments', each wrapped once more; a
loop of `f"..."` writes, whose combinators nest, wrapped it again at each
call. What was said of one object grew to hundreds of thousands of nodes,
copied at each read: most of the game's checking time, after the solver's.
Now what's said is flattened and said once: a conjunction inside a
conjunction is spliced in, what says nothing is left out, and the same
thing isn't said twice of an object.

The whole game checks in about half a second, half of it the libraries;
it took over two with 0.76.0, and minutes with the search in one method.

## Numbers too big

Fourier–Motzkin multiplies coefficients when it eliminates a variable. A
product's lemmas (docs/51) have the bounds of its factors' types in them,
up to 2^64, and two of those multiplied don't fit in the `i128`s the
solver counts in. Until now that ended the proof, as "the proof exceeded
the solver's checked integer arithmetic range": so `n == width * height`
in scope stopped the proof of `cy + 2 <= height` in a loop that had
nothing to do with it.

Now a constraint whose numbers don't fit is left out. What's left follows
from what's known, so a contradiction found without it is a real one; a
proof that fails, having left something out, still says the numbers were
why.

```rust
fn need(n: usize, x: { x: usize | x < n }) -> usize { x }

fn last(width: usize, height: usize, n: { n: usize | n == width * height }) -> usize {
    let mut s = 0;
    for y in 0..height {
        for x in 0..width {
            s = need(n, y * width + x);
        }
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&last(3, 2, 6))?; // expect: 5
    Ok(())
}
```

## Counted, not timed

Each blow-up has a test in `tests/proof_work.rs` that checks programs of
growing size and bounds the work: how many conjunctions Fourier–Motzkin
was run on, how many cases were split, and how big what's said of values
got (`t_lang::check::work`, counted per thread). The counts depend only
on the program, not on the machine, so the tests can't be flaky; and each
checks that the work grows slowly with the size, which a return of the
exponential would fail at once.

## A reference out of an enum

A refinement of what a reference points to, inside an enum's type
argument, was forgotten by a pattern that took the reference out:

```rust
#[derive(Discard)]
struct Found { squares: usize }

fn squares(
    n: usize,
    found: Option<&{ f: Found | f.squares == n }>,
) -> { k: usize | k == n || k == 0 } {
    let Some(f) = found else { return 0 };
    f.squares
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = Found { squares: 4 };
    out.line(&squares(4, Some(&x)))?; // expect: 4
    Ok(())
}
```

`f` knew what `f` points to has `squares == n`, but reading `f.squares`
read the field alone, not the struct it's in, which is what has the
refinement. Now a field read of a struct read nowhere else has what's
declared of the struct too.

## No values

`None` has no values in its `Some`, and an empty list has none at all: so
they have every refinement there. Step 60 knew that of `List::new()` by
parametricity (nothing gives it a value of `T`), but two places forgot it.
Joining branches took a branch's `None` as holding any value, so
`if b { Some(l) } else { None }` wasn't a refined `Option<List<..>>`
even when `l` was refined; and a `let` declaring a refinement of an empty
list's elements didn't know it of its value after, so iterating it knew
nothing. Now a join takes what each branch knows of its values at each
position, with an enum's of each variant where it's that variant, and a
value checked against a refinement has it:

```rust
use alloc::list::List;

fn get(n: usize, b: bool) -> Option<List<{ v: usize | v < n }>> with alloc {
    let mut l: List<{ v: usize | v < n }> = List::new();
    if n > 1 {
        l.push(1);
    }
    if b { Some(l) } else { None }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = [5u8, 6, 7];
    let found = match get(3, true) {
        Some(way) => Some(way),
        None => None,
    };
    let way = match found {
        Some(w) => w,
        None => List::new(),
    };
    for v in way.iter() {
        out.line(&a[*v])?; // expect: 6
    }
    Ok(())
}
```

## Refined places lent `&mut`

Since step 30, a field a refinement is about can be lent `&mut` straight to
a call that can't keep the reference, and the refinement is proved again
after, of what the call left, from what it ensures. Two other refined
places couldn't be: what a refined `&mut` points to was checked against
the callee's parameter type, which says nothing, and a refined `let` couldn't
be borrowed `&mut` at all. Now both are proved again after the call, the
same way:

```rust
use alloc::list::List;

fn mark(n: usize, l: &mut { l: List<u8> | l.len() == n }, v: { v: usize | v < n }) {
    l.set(v, 1);
    l[v] = 2;
    if n >= 2 {
        l.swap(0, 1);
    }
}

fn marked(n: usize, v: { v: usize | v < n }) -> { l: List<u8> | l.len() == n } with alloc {
    let mut l: { l: List<u8> | l.len() == n } = List::filled(n, 0);
    l.set(v, 1);
    mark(n, &mut l, v);
    l
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = marked(3, 2);
    out.line(&l.len())?; // expect: 3
    Ok(())
}
```

`set`, `replace`, `swap` and `index_mut` each ensure the length stays.
`push` doesn't, so:

```rust,error
use alloc::list::List;

fn grow(n: usize, l: &mut { l: List<u8> | l.len() == n }) with alloc {
    l.push(0);
}
// error: can't prove `l.len() == n` for `*l` after the call to `push`
```

A refined `let` still can't be borrowed `&mut` into a variable, or by a
call that keeps the reference: nothing would check the refinement where
that borrow ends.

## Small things

- `Iterator::skip(n)`: the values after the first `n`.
- `core::num::saturating_add` and `saturating_sub`: past the greatest or
  least value of the type, that value.
- `t fmt --check | head` no longer panics when `head` stops reading; nor
  does any other command of `t` that writes to standard output.

## Why it's sound

- **Lemma L1 (said once).** Duplicates and constants are rewritten by
  idempotence (`p && p` is `p`) and evaluation: the formula means the
  same.
- **Lemma L2 (backjumping).** Each atom and each disjunction to split
  carries the set of splits whose choices it depends on; a fact or the
  negated goal depends on none, an alternative chosen at split `d` of a
  disjunction depending on `O` depends on `O` and `d`. Invariant: what
  depends on `D` follows from the question and the choices in `D`.
  Fourier–Motzkin's derived constraints carry the union of what they're
  combined from, so a contradiction found depends on a set `D` of which the
  question and those choices can't all hold. At split `d`, if a case's
  contradiction doesn't depend on `d`, it holds whatever's chosen there:
  every case of the split is refuted. If each case's depends on `d`, the
  split is refuted by the union of theirs without `d`, and `O`, as the
  question and `O`'s choices imply one of the alternatives holds. Splits
  past the 128th get no mark, so nothing is skipped there; a product's
  lemmas depend on everything in the case.
- **Lemma L3 (left out).** Fourier–Motzkin with some derived constraints
  left out derives only what it would have with them: a contradiction
  found is one. The answer still depends only on the question.
- **Lemma T1 (flat).** What's said of the values at a position is a
  conjunction of disjunctions; splicing a conjunction into one, leaving
  out `true`, and saying a thing once are associativity, identity and
  idempotence. Equality is structural, a closed predicate by identity.
- **Lemma T2 (a field read).** What's declared of a struct where it is
  (`place_read`) holds of whatever's there (step 60's lemma A6), so of the
  struct a field read reads.
- **Lemma T3 (no values).** A value with no cases at a position has no
  values there (step 60's A2 and A3: an instantiation with no candidates
  is `false`). A join is, at each position, the disjunction of each
  branch's cases, each under its branch's condition, and an enum's are
  under the condition that it's that variant: each holds of the branch
  taken. A value checked against a refinement has it, as objects never
  change.
- **Lemma T4 (lent, proved again).** Step 30's argument: a place lent
  straight to a call-scoped `&mut` (or a lend-framed one, step 59) can't
  be read until the call's done (or the borrow ends), the callee can't
  keep the reference, and a panic ends the program. So what's declared of
  the place itself needs to hold only when it's next read, and it's proved
  then, of what's there, from what the call ensures, before anything
  assumes it (`Walk::reproving`). What's declared of positions inside it,
  the elements of a list, is still checked at the call (step 60's A3).
- **The interpreter checks** a refined variable's whole refinement each
  time it's used, not only what's carried in it: a call it's lent to can
  change it.

The adversarial tests are `tests/t/err_soundness_refined_*.t` and
`err_soundness_targ_vacuous_*.t`; the solver's are the unit tests in
`logic.rs`.

## What it doesn't do

- Iterator adapters don't carry what's known of the values they give:
  `l.iter().skip(1)`, `enumerate` and `zip` give values of `I::Item`,
  which come from `I`'s own `next`, not from an argument, so step 60's
  instantiation finds none. Saying what an associated type's values are
  is its own step.
- A part of a refined `let` (a field of it, or what's in a box in it)
  still can't be lent `&mut`: the refinement would have to be proved again
  of the whole, which nothing here does yet.
- The solver still has no learning: a conflict found in one branch of a
  split isn't remembered for the others, as a CDCL solver would.

## Decisions

- D1, a refined `let` lent straight to a call-scoped `&mut` is proved
  again after the call, as fields are; any other `&mut` of one is still
  rejected.
- D2, the checker's speed is tested by counting its work, not timing it.
- D3, `Iterator::skip`, `saturating_add`/`saturating_sub` and the `t fmt`
  pipe fix, from t-maze's list.

## Exercises

1. In L2, why must a disjunction found inside a chosen alternative carry
   that alternative's split? Write the formula for which backjumping would
   be wrong without it.
2. Backjumping skips a split's other cases when a case is refuted without
   its choice. What would a solver that *learned* the reason (as CDCL
   does) skip that this one doesn't?
3. Why may a constraint whose numbers don't fit be left out, but not one
   whose numbers do? What would a solver that rounded them instead need to
   show?
4. Why does a refined `let` lent to `clear(&mut l)`, a function that
   ensures nothing, fail, while one lent to `l.set(i, x)` checks? What does
   `set` ensure?
5. Why can't a refined `let` be lent to `fn keep<'a>(l: &'a mut List<u8>)
   -> &'a mut List<u8>`, even if nothing is written through the result?

## Reading

- Stallman and Sussman, *Forward reasoning and dependency-directed
  backtracking in a system for computer-aided circuit analysis* (1977): the
  dependencies.
- Prosser, *Hybrid algorithms for the constraint satisfaction problem*
  (1993): conflict-directed backjumping.
- Horrocks and Patel-Schneider, *Optimising description logic
  subsumption* (1999): backjumping in tableau provers, with dependency
  sets on what each branch adds.
- Marques-Silva and Sakallah, *GRASP: a search algorithm for
  propositional satisfiability* (1999): conflict analysis and learning.
- Schrijver, *Theory of Linear and Integer Programming* (1986), on
  Fourier–Motzkin elimination.
