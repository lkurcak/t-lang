# Step 81: contradictions, impls on function types, argument reborrows

Code: the proof that falls back on facts the goal doesn't share variables
with (`logic::valid`, `components`, `contradictory`, in
`src/check/refine/logic.rs`); effect sets in impl matching, coherence and
method lookup (`traits::matches`, `unify_effects`, `may_unify` in
`src/traits.rs`, `could_match_in` in `src/check/traits/methods.rs`), the
rule on an impl's effect sets (`mixed_effects`,
`src/check/traits/collect.rs`) and monomorphisation's names for function
types (`Cx::copy_name`, `src/mono.rs`); the reborrow of a `&mut` argument
(`Builder::call_into`, `src/borrowck/build.rs`); and the names messages
use (`traits::shown_trait`, `diagnostics::bare`).

Retrospective 12 found no hole in steps 79 and 80, but four things that
were wrong in smaller ways, which this step fixes together:

```text
fn absurd(_u: { u: () | false }) -> ! { loop {} }

fn small(x: { x: u32 | x > 5 }) {
    if x < 3 {
        absurd(())      // error, on 0.95: can't prove `false`
    }
}

impl Tr for fn(u32) -> u32 { .. }
let f: fn(u32) -> u32 = inc;
f.go()                  // error, on 0.95: `fn(u32) -> u32` doesn't
                        // implement `Tr`

fn by_ref(g: &mut Game) {
    put(g, 0, at(g, 1)) // error, on 0.95: cannot borrow `*g` while
                        // `*g` is mutably borrowed
}
```

The first is a branch that can't be taken, which the checker knew but
didn't use. The second is an impl no type had matched since effects came
into function types (step 7). The third is a borrow the checker made
that didn't wait for the other arguments, as one written would. And the
fourth is messages: `DiscardWhen`, a name no program can write, and
`(*r)` for `*r`.

## What can't all be true

What's known at a point of a program, the *facts*, is true on every run
that gets there (docs/08). If they can't all be true, no run gets there,
and anything at all may be assumed there: a call of a function whose
argument's type has no values, or an index out of bounds anywhere else.

```rust
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn pick(x: { x: u32 | x > 5 }, a: &[u32]) -> u32 with {} {
    if x < 3 { a[10] } else { x }
}

fn clamp(mut i: u32) -> u32 with {} {
    while i < 10 {
        i += 1;
    }
    if i < 10 { absurd(()) }
    i
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pick(7, &[1, 2]))?; // expect: 7
    out.line(&clamp(4))?;         // expect: 10
    Ok(())
}
```

The checker always proved a goal from the facts sharing variables with
it, those sharing variables with those, and so on (`connected`): the
others "can be made true on their own", so they can't help. That's true
only if they aren't contradictory. A goal like `false` shares variables
with nothing, and `a[10]`'s, `10 < a.len()`, not with `x`: so neither saw
that `x > 5` and `x < 3` can't both hold.

Now, when a proof from the connected facts fails, the rest are split
into groups that share no variables, and each group is tried on its own:
if one is contradictory, the goal holds. A proof that was found before is
found the same way, and the fallback costs only where a proof fails,
which in a program that checks is where an operator in a function `with
panic` isn't proved not to panic, or one of the checker's own guesses
(t-chess: 11 times, too few to time).

It's what an enum's refined `Discard` (docs/80) needed. A generic
`Opt<T>`'s values that hold nothing may be dropped; its `discard` must
still match the other variant, which holds a `T` to use up, and that arm
is where nothing runs:

```rust
enum Opt<T> { No, Yes(T) }

impl<T> Opt<T> {
    #[measure]
    fn is_no(&self) -> bool {
        match self {
            Opt::No => true,
            Opt::Yes(_) => false,
        }
    }
}

fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

impl<T> Discard for { o: Opt<T> | o.is_no() } {
    fn discard(self) {
        match self {
            Opt::No => {},
            Opt::Yes(_t) => absurd(()),
        }
    }
}

struct Fd { n: i64 }

fn main() {
    let _empty: Opt<Fd> = Opt::No;
    match Opt::Yes(Fd { n: 1 }) {
        Opt::Yes(Fd { n: _ }) => {},
        Opt::No => {},
    }
}
```

`absurd`'s body is a `loop {}`, which no run reaches: its argument's type
has no values, so it's never called. A program that could call it would
be refused there.

What was true and isn't any more is no fact: what's known at a loop's
head is what every iteration keeps, and what's known of a value lent
`&mut` is what the call says it left (docs/52, docs/59):

```rust,error
fn absurd(_u: { u: () | false }) -> ! {
    loop {}
}

fn main() {
    let mut i: u32 = 0;
    while i < 10 {
        if i > 5 { absurd(()) }
        i += 1;
    }
}
// error: can't prove `false` for argument `_u` of `absurd`
```

## Impls on function types

A function type's effects are a set (docs/07): `fn(u32) -> u32 with
{alloc, panic}` is `fn(u32) -> u32 with {panic, alloc}`. Finding an impl
compares types part by part, and two sets of effects were never the same
part, so since step 7 an impl for a function type was accepted and then
never found. Now two sets are the same when they have the same effects,
and an impl's effect parameter, alone, stands for any set:

```rust
trait Name {
    fn name(&self) -> u32;
}

impl Name for fn(u32) -> u32 {
    fn name(&self) -> u32 { (*self)(1) }
}

impl<effect e> Name for fn(u8) -> u8 with e {
    fn name(&self) -> u32 { 2 }
}

fn inc(x: u32) -> u32 with {} { x % 10 + 1 }

fn half(x: u8) -> u8 with panic { 200 / x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f: fn(u32) -> u32 = inc;
    let g: fn(u8) -> u8 with panic = half;
    out.line(&f.name())?; // expect: 2
    out.line(&g.name())?; // expect: 2
    Ok(())
}
```

An impl for `fn(u32) -> u32` isn't one for `fn(u32) -> u32 with panic`:
matching is by type, never by what could be passed where (docs/06a). Nor
is it one for `fn(u32) -> u32 with e` in generic code, as `e` may stand
for a set with effects in it. An impl's type may say a set in full, or
as one effect parameter alone, but nothing between:

```rust,error
trait Name {
    fn name(&self) -> u32;
}

impl<effect e> Name for fn(u32) -> u32 with {panic, e} {
    fn name(&self) -> u32 { 1 }
}
// error: a set of effects in an impl's type is written out in full, or is one effect parameter alone, not `{panic, e}`
```

`fn(u32) -> u32 with panic` would match that with `e` either `{}` or
`{panic}`, and which `e` an impl is at must be a function of the type,
for its associated types to be (docs/06b).

Coherence uses the same rule, so impls that could be for one function
type overlap: an impl for `fn(u32) -> u32 with e` overlaps one for
`fn(u32) -> u32 with panic`, and a blanket `impl<T: Tr> B for T`
overlaps `impl B for fn(u32) -> u32` where `fn(u32) -> u32: Tr`. On 0.95
neither was refused, harmlessly, as neither impl was ever found.

## A `&mut` argument's reborrow waits

A call works out its arguments in order, but a borrow of a place, `&mut
v` or `&mut *g`, is only taken once all of them are, just before the
call (docs/06, "args first"): `v.push(v.len())` is `let n = v.len();
v.push(n)`. A `&mut` reference used as an argument is reborrowed, `&mut
*g`, so that `g` can be used again after the call; but that reborrow was
taken where `g` was, before the other arguments:

```rust
#[derive(Discard)]
struct Game { b: [u8; 4] }

fn at(g: &Game, i: { i: usize | i < 4 }) -> u8 { g.b[i] }

fn put(g: &mut Game, i: { i: usize | i < 4 }, v: u8) {
    g.b[i] = v;
}

fn copy_one(g: &mut Game) {
    put(g, 0, at(g, 1));
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut g = Game { b: [1, 2, 3, 4] };
    copy_one(&mut g);
    out.line(&g.b[0])?; // expect: 2
    Ok(())
}
```

Now the reborrow of a `&mut` at a place, a variable, a field or a `*`,
waits as the one written does. A borrow an earlier argument keeps is
still live at the call, so two `&mut`s to one value are still refused:

```rust,error
#[derive(Discard)]
struct Game { b: [u8; 4] }

fn cell(g: &mut Game) -> &mut u8 { &mut g.b[0] }

fn both(g: &mut Game, c: &mut u8) {
    g.b[0] = 1;
    *c = 2;
}

fn play(g: &mut Game) {
    both(g, cell(g));
}
// error: cannot borrow `*g` as mutable more than once at a time
```

This isn't Rust's two-phase borrows, where a `&mut` is taken first and
only *activated* at the call: in T the borrow is taken at the call, and
what the arguments do before it runs first, as written.

## Messages

- An impl of `Discard` for a refined type is one of `DiscardWhen`
  inside (docs/80), but messages say `Discard`: "`discard` may panic, but
  `Discard`'s `discard` can't", "`Discard` is `core`'s and `List` is
  `alloc`'s". `impl<T> Discard for { t: T | true }` gets the message any
  impl of `Discard` for a type that isn't a struct or enum gets, not one
  calling `T` `core`'s.
- A place behind a reference is named `*r` where it stands alone, as in
  "`*r` is dropped here" or "it's in `*o`, whose type has a refinement",
  and `(*o).a` where more follows.

## Why it's sound

> **Lemma P3a (contradictions).** The facts at a point are true on every
> run that reaches it (the walk's invariant, docs/66). If a subset of
> them is unsatisfiable, no run reaches it, so every goal there holds of
> every run that reaches it, vacuously. `contradictory` refutes a group
> with the procedure that proves goals (Fourier–Motzkin over cases), and
> only its `true` is used: running out of cases or arithmetic says
> nothing.

> **Lemma P3b (groups).** Facts that share no variables, a product
> sharing its factors', constrain disjoint sets of variables: if each
> group can be made true, all can, at once. So the facts are
> contradictory exactly when one group is, and trying each alone loses
> nothing, up to each one's limits. The groups and their order are a
> function of the facts, so the answer depends on nothing but the
> question (docs/61).

> **Lemma F2a (matching sets).** An impl's type has, in each set of
> effects, only effects, or one of its parameters alone
> (`mixed_effects`). Matching it to a type binds that parameter to the
> type's set, or compares the effects with the type's set, which
> `Type::effects` keeps sorted and without repeats, so equal sets are
> equal. So matching is still a function of the type: at most one impl
> matches, as coherence (F2b) keeps, and which parameter is what is
> decided by the type. It commutes with substitution: a set of effects
> alone is itself at every instance, and a parameter of generic code in
> the type's set is rigid, matched only by an impl's parameter, so an
> impl found in generic code is found at every instance (lemma N2 of
> docs/73), and one not found there is left to monomorphisation, as for
> any type (docs/06b).

> **Lemma F2b (overlap).** `unify` of two sets binds a variable alone to
> the other set, and says two sets of effects (or rigid parameters) are
> the same only if equal; any other two it says may be the same, binding
> nothing. So it never says two impls are apart where a type could match
> both: coherence and the negative reasoning that uses it (`refuted`,
> `alone`) are as sound as before, and only more cautious where a set
> isn't known yet.

> **Lemma F2c (copies).** Monomorphisation names the copy of an impl's
> method by its type, and a function type's name now has its effects:
> two impls for `fn(u32) -> u32` and `fn(u32) -> u32 with panic` get two
> copies, not one shared. (Writing the step found this: the second call
> ran the first impl's `out`, which the interpreter caught as a `bool`
> that was a `u32`; `impl_fn_type_effects.t`.)

> **Lemma A4 (the reborrow waits).** A `&mut` argument at a place `p`
> that the checker didn't coerce was lowered as a use of `p`, which for a
> `&mut` is the reborrow `&mut *p` (`use_place`), there and then. It's
> now `Arg::Borrow(*p)`, exactly what `&mut *p` written is: the same MIR
> statement, taken at the same point, just before the call. So the
> argument of docs/06 for args first holds as it is: finding `*p` is
> following fields and `*`s, which no other argument can observe, and the
> borrow checker sees every borrow and use in the order they run.

The adversarial tests are `tests/t/err_soundness_contradiction_*.t`
(facts from before a loop, a value lent `&mut`, two paths joined, a
closure's parameter, an arm nothing rules out: none contradictory, all
refused), `err_soundness_impl_fn_rigid_effect.t`, `err_impl_fn_*.t`, and
`err_soundness_reborrow_arg_*.t`. Switching off the fallback, the effect
sets in matching, the effects in a copy's name, or the waiting reborrow,
each fails tests.

## What it doesn't do

- **Prune paths.** A branch whose facts contradict is still walked, and
  its proofs still tried first from their own facts: only a failed one
  asks whether the rest are contradictory.
- **Function types generic over lifetimes.** Matching an impl ignores
  what a function type is generic over (`fn(&u32) -> u32` is generic
  over its reference's), as before: so an impl's associated type still
  can't use a parameter that's only inside a function type (docs/06b).
- **Shared references.** A `&T` argument is copied, which waits for
  nothing and needs nothing to.

## Decisions

- **D1: any goal, on failure.** Not only goals with no variables (which
  leaves `a[10]` on an impossible branch refused), and not pruning
  branches (a proof per branch whether needed or not, and blind to
  contradictions that come from calls).
- **D2: mixed sets refused in impl types.** Not accepted and never
  matched.
- **D3: everywhere.** `*r` in every message where a name stands alone;
  the reborrow waits for any `&mut` at a place, not only a variable.
- From retrospective 12: `Option` gets no refined `Discard` (no program
  needs it; with P3 one can write its own, as `Opt` above), and t-libs'
  hash map and graph get refined impls of their own in this step.

## Exercises

1. Write a function `fn first(l: { l: &[u32] | l.len() > 0 }) -> u32`
   whose body indexes `l[5]` in a branch that `l.len() > 0` doesn't rule
   out. Then add a condition to the branch that does.
2. Why must `contradictory` split the facts into groups, rather than
   refute them all at once? (Think of `MAX_CASES` and disjunctions.)
3. `impl<effect e> Tr for fn() -> u32 with e` and `impl Tr for fn() ->
   u32` overlap. Why does coherence say so, and what would go wrong if it
   didn't?
4. Why does `could_match_in` say a set with an effect variable in it
   could be any set, but `matches` says it's none?
5. With the reborrow waiting, `put(g, 0, { g = other; 1 })` writes to
   `*other`. Why is that what args first means, and why couldn't it be
   observed before this step?

## Reading

- Bradley and Manna, *The Calculus of Computation* (2007): decision
  procedures for linear arithmetic, and why a conjunction over disjoint
  variables is satisfiable when each part is.
- Meng and Paulson, *Lightweight relevance filtering for
  machine-generated resolution problems* (2009): choosing the facts a
  proof may need by shared symbols, and what it loses.
- Leijen, *Koka: Programming with Row-polymorphic Effect Types* (2014):
  effect sets that are equal up to order, and variables standing for the
  rest of a row.
- Rust RFC 2025, *Enable two-phase borrows for vector method calls*
  (2017): the other way to let `v.push(v.len())` through.
