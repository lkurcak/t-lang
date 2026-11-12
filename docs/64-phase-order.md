# Step 64: types, borrows, then proofs and effects

Code: `Checker::body`, `Checker::reveal_hidden` and `Checker::keep_mir`
in `src/check.rs`; `Checker::solve_types` and `Checker::solve_effects` in
`src/check/unify.rs`; `Checker::settle_closures` and
`Checker::resettle_closures` in `src/check/closures.rs`;
`Checker::prove_all` and `Checker::waits_for_effects` in
`src/check/traits/prove.rs`; `Body::map_types` in `src/mir.rs`; and where
`Traits::implements` matches a derived marker's impl, in `src/traits.rs`.

The refinement checker walks the AST. Four of the last five releases that
fixed it fixed the same kind of hole: a way of getting at a place that the
walk had no case for. A variant's field, an array's element, a binding of
an or-pattern, a temporary. MIR has one grammar of places, a local and then
fields, variants and `*`s, and every desugaring is already done by the time
it's built. So the plan (retrospective 6, and the design pass after it) is
to check refinements on MIR, as Flux does for Rust, and retire the walk.

The MIR, though, was built last. Proofs decide effects: `x / 2` can't
panic, so `|x: u8| x / 2` is a closure `with {}`. Effects decide closures'
signatures, and the borrow checker, which builds the MIR, needs those. So
the order was refinements, effects, closures, the borrow checker. A
checker on MIR can't run where the refinement checker ran.

This step changes the order and nothing else. The AST walk is unchanged,
and no program is accepted or refused that wasn't before. For each
function:

1. its types, as before;
2. its closures: what each captures, how, and so which `Fn` traits it
   implements, with its effects still a variable;
3. the trait obligations that waited for its types, except those that wait
   for its effects;
4. the borrow checker, which builds its MIR, checks it and keeps it;
5. its refinements, which say which operators can't panic;
6. its effects, the least each closure can have, and then they go into
   the MIR's types, the closures' signatures and what its opaque types
   stand for;
7. the obligations that waited for effects.

Step 65 puts a refinement checker on MIR at 5, beside the walk, and
compares the two. Step 66 makes it the one.

```rust
#[derive(Discard, Copy where F: Copy)]
struct Pair<F: Fn(u8) -> u8> { f: F, n: u8 }

fn use_pair<F: Fn(u8) -> u8 + Copy>(p: Pair<F>) -> u8 { (p.f)(p.n) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = Pair { f: |x: u8| x / 2, n: 9 };
    out.line(&(use_pair(p) + use_pair(p)))?; // expect: 8
    Ok(())
}
```

Here `p` is copied, not moved, so it can be used twice. That's the
borrow checker's business, and the MIR's: a `Copy` value is read where
it's used, anything else is moved out. `Pair<{closure}>` is `Copy` if the
closure is, and the closure fits `Pair`'s bound only if it can't panic,
which a proof says. The next section is about why the borrow checker can
know `p` is `Copy` before that proof.

## What waited for effects

Effects are types of a kind of their own, and a closure's are a variable
until the end of its function (docs/07-effects.md). Before this step every
variable was solved by the time anything looked. Now steps 2 to 5 look at
types whose effects may be variables, so each of them has to not care.

- **Closures.** How a closure captures a variable depends on what its body
  does with it, and which `Fn` traits it implements on that and on whether
  what it owns may be thrown away. Neither is about effects. Its signature
  has its effects in it, so it's stored with the variable in, and
  `resettle_closures` puts the set in once it's known.
- **Calls through a reference.** `r(8)`, with `r: &fn(i64) -> i64 with e`
  for an `e` still to settle, is a call of what `r` points to. That depends
  on `r`'s type but for its effects. It used to skip any type with a
  variable in it, which now would have been every such call
  (`unify::has_type_vars`).
- **Obligations.** Most are proved where they're made; those whose types
  weren't known yet wait for the end of the function. Some can't be
  proved before effects: `fn() -> i64` has no effects, and an impl for it
  isn't one for `fn() -> i64 with panic`. An obligation of a type with an
  effect variable in it, or with a closure whose signature or captures
  have one, waits for step 7 (`waits_for_effects`). The rest are proved
  at step 3, before the borrow checker, so a missing impl is still
  reported before a borrow error.
- **The borrow checker.** Effects aren't values and hold no references.
  Relating two effect sets says nothing (`Regions::relate`), relating two
  function types doesn't look at their effects (`relate_fns`), and an
  effect variable has no regions to outlive anything. The MIR it builds
  has effect variables in its types, and `keep_mir` puts in the sets
  before it's kept for running, so what runs is what ran before.
- **Copy and discard.** The borrow checker asks whether a value's type is
  `Copy`, to read it rather than move it, and `Discard`, to let it be
  thrown away. The next section is about why that doesn't wait for
  effects.

What `Pair` shows didn't hold at first. A derived impl of a marker,
`Discard`, `Copy`, `Send` or `Sync`, was matched with its type's own
equality bounds checked, and `F: Fn(u8) -> u8` has one: that calls of `F`
are `fn(u8) -> u8`, with no effects. So `Pair<{closure}>: Copy` asked
what the closure's effects were. Asked at step 4, with them still a
variable, the answer was no, and the program above was refused: "use of
moved value `p`".

## A marker takes its type's bounds for granted

A derived marker impl needs only more of the same trait of its type's
parameters, the least that makes each field implement it. Lemma D1 of
docs/17-retrospective-1.md proves that, and it's what keeps a proof of
`Discard` from wandering into a program's own traits. The equality bounds
broke it: through `Pair`'s, a proof of `Copy` reached `Fn`'s signature,
and through that a closure's effects.

They're taken for granted now. They are the type's, so they hold of every
type that's well-formed, and every type a program makes is checked to be,
effects included: `Pair { f: |x: u8| x / 2, n: 9 }` is where the closure
is checked against `Pair`'s bound.

```rust,error
#[derive(Discard, Copy where F: Copy)]
struct Pair<F: Fn(u8) -> u8> { f: F, n: u8 }

fn main() with panic {
    let _p = Pair { f: |x: u8| x / (x - x), n: 9 };
}
// error: this closure may panic, but `Pair` needs calls of it to fit `fn(u8) -> u8`, which can't panic
```

## Which error comes first

A program with one error gets the same error. One with two may get the
other one first, as the order of checks changed. Types and trait
obligations still come first. Then come borrow errors, which used to come
last, and then refinements and effects, which used to come first. Flux
also checks refinements of Rust that rustc has borrow-checked, and there's
little point in proving what a value is when the program reads it while
it's mutably borrowed:

```rust,error
fn f() -> u8 with {} {
    let a: [u8; 50] = [0; 50];
    let mut i: usize = 0;
    let mut bump = || i = i + 100;
    while i < 10 invariant i < 50 {
        bump();
        a[i];
    }
    0
}

fn main() {}
// error: cannot use `i` while `i` is mutably borrowed
```

The refinement checker used to report four errors here: three
refinements it couldn't prove and that `bump` may panic. Called before
the loop, `bump` holds `i` only while it runs, and those four are what's
reported, as before.

An unused variable whose type isn't `Discard` is said to be one at step
2, before the borrow checker's errors, which are about the same variable.

## Why it's sound

What's to prove is that the change is only one of order: the same
programs are accepted, the same MIR runs, and the same operators are
proved. Each step reads only what the steps before it settled.

> **Lemma P1 (effects settle the same).** The least solution of a
> function's effect variables is worked out from its `Within`s and its
> sites, less the operators proved. Only checking the body and the
> refinement checker make those, and neither reads anything steps 2 to 4
> make: the refinement checker asks only `Int`, `Signed` and `Unsigned`
> of types, and closure kinds and the MIR don't come into it. So each
> variable gets the same set.

> **Lemma P2 (nothing before effects reads them).** Steps 2 to 4 give the
> same results whatever the effect variables turn out to be. Captures and
> closure kinds are of what bodies do and of `Discard` (P3); calls of what
> isn't a function type are of types but for their effects; the
> obligations proved at step 3 have no effect variable in them, nor in a
> closure in them; the borrow checker relates no effect sets and finds no
> regions in effect variables.

> **Lemma P3 (markers don't depend on effects).** Whether a type is
> `Discard`, `Copy`, `Send` or `Sync` doesn't depend on any effect set in
> it. By induction on the proof, as for D1: the built-in impls ignore a
> function type's effects, a closure's are of its captures' types, a
> written `Discard` may need only `Discard` of its type's parameters, and
> a derived one only its trait of its fields' types, now that its type's
> equality bounds are taken for granted. Taking them for granted changes
> no answer for a type that's well-formed, and every type in a program
> that's accepted is: a type is checked to be where a value of it is made,
> its bounds' effects as `Within`s at step 6.

> **Lemma P4 (the same MIR runs).** `keep_mir` replaces each type `t` in
> the MIR with its solution, which is idempotent, so the result is what
> building it after step 6 would have given: the same blocks, with the
> same types. The closures' signatures and what opaque types stand for
> are solved the same way.

So every accepted program has the same MIR, effects and proofs as before.
That was checked too, not only argued: for each of the 2,275 programs in
`tests/t` and `examples`, what `t mono` prints (the MIR, types and all)
and what `t build --emit-c` writes (which leaves out the checks of the
operators proved) are the same as with 0.79.0, byte for byte, and so are
the errors of those refused, once the two tests above had one error each.
A refused program is refused, maybe for another of its errors.

## Decisions

- **Borrow errors before refinement errors.** The alternative was to run
  the borrow checker early but hold its error until refinements and
  effects had been checked, keeping the old order. But then step 65's
  checker would have to work on MIR the borrow checker had refused.
- **Implied bounds, not a rule against them.** Refusing `Fn` bounds on
  types would also have broken the cycle; nothing outside the tests has
  one. Taking a marker's equality bounds for granted is what D1 already
  claimed, and needs no new rule.
- **Not provisional.** The other way to keep the old rules: answer
  `Copy`/`Discard` with effects as variables, ask again once they're known,
  and redo closures, MIR and the borrow checker if an answer changed. A
  path that's almost never taken is a path that's almost never tested.

## What it doesn't do

- The refinement checker still walks the AST (step 65 adds one on MIR;
  step 66 retires the walk).
- The borrow checker's results, which loans are live where, aren't kept
  for refinements to use yet. A closure that captures a variable by
  `&mut` still makes the variable unknown to the refinement checker for
  the whole function (`unstable`), rather than until the closure's borrow
  ends. (Since step 69 it's forgotten where it's captured, which, as the
  borrow checker lets nothing touch it until the borrow ends, comes to
  the same: docs/69-loans.md.)

## Exercises

1. Find an effect variable in a MIR type before `keep_mir`: write a
   function that coerces a closure to `fn(i64) -> i64 with e` for an
   effect parameter `e`, and print the local's type in `borrowck::check`.
2. Why does `waits_for_effects` look at a closure's captures' types, not
   only its signature? Write the impl that would make it matter.
3. P3 says an accepted program's types are all well-formed. Where does
   the checker check `Pair<{closure}>` is, and where does the `Within`
   for its bound come from?
4. Turn `resettle_closures` off and run the tests. Which fail, and why
   there?

## Reading

- Nico Lehmann, Adam Geller, Niki Vazou, Ranjit Jhala, *Flux: Liquid
  Types for Rust* (PLDI 2023): refinements checked on MIR, after rustc's
  borrow checker.
- RFC 2089, *Implied bounds* (Rust): a type's bounds taken for granted
  wherever the type is well-formed.
- The rustc dev guide, *The MIR*: which passes run before borrow
  checking, and why typeck's obligations are all solved first.
