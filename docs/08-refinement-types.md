# Step 8: refinement types

Code: `src/check/refine.rs` (the pass over each body: facts, what each
refinement needs, which operators can't panic), `src/check/refine/logic.rs`
(the logic, and the procedure that decides it), `src/check/effects.rs`
(operators' sites, taken out when proved), `core/num.t` and `core/ops.t`
(arithmetic written in T and proved total), and `src/interp.rs`
(`Refined::check` and `Interp::not_proved`, which check it all at run
time).

```rust
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn clamp(x: int, hi: { h: int | h >= 0 }) -> { r: int | r <= hi } {
    if x < hi { x } else { hi }
}

fn main() {
    print(div(10, 3)); // expect: 3
    let k = clamp(25, 7);
    print(k); // expect: 7
    if k > 0 {
        print(div(100, k)); // expect: 14
    }
}
```

A *refinement type* is a type with a condition on its values: `{ d: int |
d > 0 }` is the `int`s greater than 0. `d` names the value, and the
condition after `|`, the *predicate*, says what's true of it. `div`
divides by `d`, and `/` panics only on 0, or on the least `int` divided by
-1, neither of which `d` can be, so `div` can't panic, and says so: no
`with panic`. Its callers have to show that what they pass is positive.
`div(10, 3)` is, and so is `div(100, k)` inside the `if`: the checker
knows `k > 0` there.

Step 7 made arithmetic say `with panic`, which was honest but meant most
functions said it. Refinements let a function say what it needs of its
arguments and promise of its result, and the checker uses what's known
to prove an operator can't panic: then it has no `panic` at all.

## Where a refinement can be

The whole type of a parameter, of a result, or of a `let` of one
variable, and only of an `int` or a `bool`. A result's predicate can name
the parameters, as `clamp`'s names `hi`; a parameter's, those before it;
a `let`'s, the variables in scope. None can name a `mut` variable: which
of its values would it mean?

Not inside another type (`Option<{ v: int | v > 0 }>`), not a closure's
parameter, and not in a trait's methods, nor an impl's of a trait:

```rust,error
trait Half {
    fn half(self, by: { b: int | b > 0 }) -> int;
}
// error: refinements can't be in a trait's methods yet
```

Each of those would need refinements inside other types, or an impl's
method to ask no more than its trait's, which is more to get right, and
can come later without changing what's here.

## What a predicate can say

Literals, names, `+`, `-`, `*` by a constant, comparisons, `&&`, `||` and
`!`: *linear integer arithmetic*. `v * v < 10` isn't linear, and is an
error, as is `/`. Integers in a predicate are the mathematical ones, so
`v + 1 > v` is always true there; that an `int` is between the least and
the greatest is just another fact.

That's a small logic on purpose: the question "does this follow from
that?" can always be answered for it (it's *decidable*), so the checker
never has to guess, and the procedure that answers it is short enough to
read and trust.

## What's known: facts

The checker walks each body knowing a list of *facts*, and adds to it:

- a refined parameter's predicate, from the start;
- a refined function's result's predicate, after a call of it, of the
  arguments it got: `clamp(25, 7)` gives `k <= 7`;
- a condition, inside the branch it decides: in `if k > 0 { .. }`, `k >
  0`; after `if d == 0 { return 0; }`, `d != 0`, as the other branch
  doesn't come back; on the right of `d != 0 && a / d > 1`, `d != 0`;
- what a `match` on an `int` matched, and what its earlier arms didn't;
- what a `let` or an assignment gives a variable: after `let y = x + 1;`,
  `y == x + 1`;
- what an operator gave, as it'd have panicked otherwise.

```rust
fn guarded(a: int, d: int) -> int {
    if d == 0 || d == -1 {
        return 0;
    }
    a / d
}

fn small_inc(n: int) -> int {
    match n {
        0..=9 => n + 1,
        _ => 0,
    }
}

fn main() {
    print(guarded(10, 3)); // expect: 3
    print(small_inc(3));   // expect: 4
}
```

Facts are about *values*. When a variable gets a new value, it stands for
a new name in the logic (*static single assignment*): what was known of
the old value stays true, of the old value. So:

- An assignment gives a variable a new value: after `d = 0`, what was
  known of `d` was of its old one.
- A loop gives each variable it assigns a value nothing is known of at
  the start of each time round, as it may be any time round. Without a
  `break`, its condition is false after it.
- After an `if`, a variable a branch assigned is whichever value the
  branch taken gave it.
- A `mut` variable that can change some other way, through a `&mut`, a
  method's `&mut self` or a closure that captures it, gives a value
  nothing is known of each time it's read.

A `let mut` with a refinement keeps it: every assignment must have it,
inside a closure too, and it's known of every value the variable has,
loops and all. That's how a loop says what it keeps true, its *loop
invariant*:

```rust
fn count(n: int) -> int {
    let mut i: { v: int | v >= 0 } = 0;
    while i < n {
        i = i + 1;
    }
    i
}

fn main() {
    print(count(5)); // expect: 5
}
```

`i + 1` can't overflow, as `i < n`, and `n` is an `int`; it's at least 0,
as `i` is. A refined variable can't be borrowed `&mut`: a write through
the reference could break its refinement, and nothing would check.

What a closure's body knows is what's known where it's made, which still
holds when it runs, as facts are about values that can't change: a
variable the body reads either can't change or gives a value nothing is
known of. What its body learns stays in it.

## Operators that can't panic

Each `+`, `-`, `*`, `/`, `%`, unary `-` and `+=` is a question for the
checker: can it overflow, or divide by zero, given what's known? If it
can prove not, the operator has no `panic`. That's every operator on
literals, `-5` and `x / 2`, which step 7 had special cases for, and much
more. `core` uses it: `Range::next` adds one to what's less than the
range's end, and `checked_add`, `checked_sub` and `checked_div` are
written in T with the operators, each proved total. The checker takes
only `swap`, `panic` and `checked_mul` on trust, which is the one that
would need products of unknowns.

When it can't prove an operator safe, nothing is wrong: it may panic, as
in step 7, and its function says `with panic`. The checker isn't always
right that it can't: `x * x` for `0 <= x < 100` can't overflow, but
that's not linear.

```rust,error
fn square(x: { v: int | v >= 0 && v < 100 }) -> int { x * x }
// error: `*` may panic, but `square` doesn't say `with panic`
```

## Where refined values go

A value with a refinement is a value of its base type, so it can go
anywhere one can; it just loses what's known of it, which is always
sound. The other way, a value is checked against a refinement wherever
step 7 checks effects against a type: an argument, a return, a `let` with
a type. That's the only subtyping: `{ v: int | v > 1 }` is a `{ v: int |
v > 0 }`, at those places, as one implies the other.

A function with a refined parameter can't be used as a value, as `apply`
couldn't check what it passes:

```rust,error
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn apply(f: fn(int, int) -> int, d: int) -> int { f(1, d) }

fn main() {
    print(apply(div, 0));
}
// error: `div` can't be used as a value: its parameter `d` has a refinement, which a call through a value wouldn't check
```

A refined *result* is fine as a value: forgetting what's known is sound.

There's no way to say "trust me": when the checker can't prove something
true, a branch on it (`if d > 0 { .. } else { .. }`) makes it known, and
the `checked_*` functions give an `Option`. An unchecked assumption would
need `unsafe`, which T doesn't have yet.

## Deciding what follows

`refine/logic.rs` decides whether a goal follows from the facts. It asks
the opposite: can the facts and the goal's negation all hold? It splits
that into cases, one for each way the `||`s go, and refutes each, a list
of linear inequalities, by *Fourier–Motzkin elimination*: it removes the
variables one by one, combining each upper bound on one with each lower
bound, until only constants are left, true or false. For integers it
rounds as it goes: `2x <= 3` is `x <= 1`, which no integer notices.

It's *sound* and *incomplete*: when it says a goal follows, it does, but
it may fail to see that some do. A contradiction that only integers have,
it can miss. That's the right way round to be wrong: a weaker procedure
rejects more programs and never accepts a wrong one, so making it smarter
later (with the Omega test, which is complete) can only accept more.
Every step is bounded, with checked `i128` arithmetic, and going past a
bound is a "don't know": the answer depends only on the question.

## Why it's sound

> Whenever a variable with a refinement gets a value, the refinement is
> true of it, and an operator proved not to panic doesn't.

The checker's facts hold of the values the program has, at each point:
that's an induction on how the program runs. A fact is added only where
it's proved, where a condition is true, after an operator returned, or
from a variable's refinement, which every assignment proves. What a
program variable stands for changes whenever its value may: at an
assignment, at the top of a loop that assigns it, after a branch that
does, and on every read if it can change some other way. A predicate
names only what can't change. A refined function's result has its
refinement by induction on how deep the calls go, as for effects:
assuming a recursive call returns what its function promises is fine, as
a call that doesn't return gives nothing to count on. And every call that
skips the check, through a function value or a trait, is of a function
with nothing to check. The procedure is sound, so each goal proved is
true. Refinements are erased before monomorphisation, so nothing else
changes.

The interpreter checks it as it goes: every refinement at each call, each
return, each refined `let` and each assignment to one, and every operator
the checker proved, which must not panic. Either failing stops it with
"type soundness violated".

## Design decisions worth noticing

- **Refinements aren't in `Type`.** Unification, traits and
  monomorphisation never see one, so step 8 couldn't break them.
- **Operators are proved automatically**, not on request, so arithmetic
  is total wherever it can be shown to be, with no new syntax.
- **Loop invariants are written, not inferred**, as the type of a `let
  mut`. Inferring them (Liquid types) needs a fixed point over guessed
  predicates, which can come later.
- **A sound, incomplete procedure, written here** rather than an SMT
  solver: what the checker trusts stays small and deterministic.
- **No escape hatch.**

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| refinements on struct fields, facts through field reads | needs facts about places, not just variables | 8.1 ([docs](08.1-field-refinements.md)) |
| refinements inside other types, `Option<{ v: int \| v > 0 }>` | needs subtyping inside type constructors, and refinements on type parameters | — |
| refined trait methods | an impl's must ask no more and promise no less | — |
| `&mut` to a refined value | writes through it must keep the refinement | — |
| functions in predicates (`r.start() <= r.end()`) | needs pure functions, *measures* | — |
| inferring loop invariants | Liquid types' fixed point | — |
| `/`, `%` by a constant in predicates | Presburger arithmetic, a complete procedure | — |
| indexing that can't panic | there are no arrays yet | `alloc` |
| `assume` | needs `unsafe` | — |

## Exercises

1. Write `fn abs(x: { v: int | v > -9223372036854775807 - 1 }) -> { r: int
   | r >= 0 }` without `with panic`. Why does the parameter need its
   refinement?
2. Why can't a refinement name a `mut` parameter? Write a function where
   allowing it would make the checker prove something false.
3. In `Walk::leaf`, return what the variable stood for even if it can
   change other than by assignment. Which test catches it, and how?
4. `count` above works with `i: { v: int | v >= 0 }`. What goes wrong with
   a plain `let mut i = 0;`? What would inferring the invariant need?
5. Find two facts the solver can't combine though they contradict, using
   only integers. (Hint: `2x == 2y + 1`.) Is that a soundness problem?

## Reading

- Freeman and Pfenning, *Refinement Types for ML* (1991), where the term
  starts.
- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008), refinements with
  inferred predicates, and Vazou et al., *Refinement Types for Haskell*
  (2014).
- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust* (2023):
  refinements with ownership, much like here.
- Swamy et al., *Dependent Types and Multi-Monadic Effects in F\** (2016),
  for where refinements lead.
- Dantzig and Eaves, *Fourier–Motzkin Elimination and Its Dual* (1973),
  and William Pugh, *The Omega Test* (1991), for deciding linear
  arithmetic over the integers.
