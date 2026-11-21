# Step 90: conditional effects

Code: `effect_conds` and `Checker::cond_refinements` in
`src/check/refine.rs`, which check what a signature says; `Walk::cut_effects`
there and `Walk::operator_proof` (`src/check/refine/diagnostics.rs`),
which find what can't happen at each site; `Checker::cut_sites`, which
takes it out, and `Checker::body_effects` (`src/check/effects.rs`), which
checks what's left; `exhaust::variant_inhabited`; and
`Interp::allowed_effects`, which checks it all as it runs.

```rust
use alloc::list::List;

fn sure(x: i64) -> Result<i64, !> { Ok(x) }

/// At most `limit`, or a panic if `x` is more.
fn capped(x: u8, limit: u8) -> u8 with panic if x > limit {
    if x > limit {
        panic("too big");
    }
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&sure(1).unwrap())?; // expect: 1
    let mut l = List::new();
    l.push(2i64);
    out.line(&l.pop().unwrap())?; // expect: 2
    out.line(&capped(3, 10))?;    // expect: 3
    l.discard();
    Ok(())
}
```

`main` unwraps twice and calls something that may panic, and says it
can't panic: which the checker checks. Until now an operator was the one
thing whose effect depended on what was known where it was: `x + 1`
panics only where it isn't proved not to (docs/08-refinement-types.md). A
call had what its callee's signature said, whatever the caller knew. So
`unwrap` panicked even of a `Result<i64, !>`, which can't be an `Err`,
and of a list just pushed to, and `core` had a separate `into_ok` that
couldn't.

Now a function can say *when* it has an effect: `with panic if x > limit`.
A call has it only where its condition isn't ruled out. That's the
operators' rule, for every function: `capped(3, 10)` can't panic, as
`3 > 10` is false, and `capped(n, m)` may, unless something says `n <=
m`. `unwrap` says `with panic if self.is_err()`, so it can't panic of a
`Result<i64, !>`; `Option`'s says `with panic if self.is_none()`, and
`pop` says when it gives a `Some`, so the second `unwrap` can't either.

## Saying it

A condition follows an effect, after `if`: `with panic if x > limit`, or,
of a set, `with {alloc if l.len() == l.capacity(), panic}`. It's a
predicate, written as `ensures` is, of the function's parameters *as
they're given*: a `mut` parameter's name is what it was at the call, and
what's behind a `&mut` what was there then, so `old(..)` isn't needed, or
allowed. Not of the result, which a call that panics doesn't give.

`panic`, `alloc` and `global` can have one. Not `unsafe`, which says what
a caller must keep (docs/09-unsafe.md), not something the body does; nor
an effect parameter or an associated effect, which are what's given. An
effect is said once: `||` joins two conditions.

The function's type is what it was: `fn(Option<T>) -> T with panic`. A
function used as a value has every effect it says, whatever their
conditions, where what it'll be called with isn't known: given to a
function that calls it, say. That's the same weakening as using a
function that can't panic where one that may is wanted
(docs/07-effects.md). Where the checker knows which function a value is,
a call of it is a call of that function (docs/78-callable-values.md), and
has its effects as one does. Closures, and functions
whose effects are inferred, have no conditions: they have what their
bodies' sites have, and a call in one whose condition is ruled out has
none.

Only a function's own, or an inherent method's: a trait's method can't
say when it has an effect, nor an impl's of one, nor an intrinsic, an
`extern fn` or a const.

## Checking the body

A function that says `with panic if P` may panic only where `P` held at
the call. Each place in its body that may panic, a *site*, is checked:

- an operator must be proved not to overflow where `P` didn't hold:
  `fn add(a: u8, b: u8) -> u8 with panic if a > 255 - b { a + b }`;
- a call with a condition of its own must be one whose condition follows
  from `P`, wherever it's made;
- any other, a `panic(..)` say, must be where `P` holds.

```rust,error
fn bump(x: u8) -> u8 with panic if x > 10 { x + 250 }

fn main() with {} {
    let _v = bump(7);
}
// error: `+` may panic when `x > 10` doesn't hold, but `bump` says `with panic if x > 10`
```

And of what it was given, not what it has later:

```rust,error
fn zero(mut x: u8) -> u8 with panic if x == 0 {
    x = 0;
    if x == 0 {
        panic("zero");
    }
    x
}

fn main() with {} {
    let _v = zero(1);
}
// error: `panic` may panic when `x == 0` doesn't hold, but `zero` says `with panic if x == 0`
```

## Values no type can hold

A `Result<i64, !>` can't be an `Err`: there's nothing to put in it.
Matching knew that already: `match r { Ok(x) => x }` needs no `Err` arm.
Now the refinement checker knows it of every value of such a type, by the
same function (`exhaust::variant_inhabited`): the variant can't be one
with a field whose type has no values, as `!`, an empty enum, or a struct
or tuple holding one. So `r.is_err()` is false of it, and `unwrap`'s
condition is ruled out.

Only where the type is known: code generic in an error type `E` can't
know it's `!`, which a call may make it.

```rust,error
fn get<E: Discard>(r: Result<i64, E>) -> i64 with {} { r.unwrap() }

fn main() with {} {
    let _v = get::<!>(Ok(1));
}
// error: `unwrap` may panic, but `get` doesn't say `with panic`
```

`into_ok`, which was for this, is gone: `unwrap` does it. The
`unwrap_infallible` warning stays for `unwrap_or` and `unwrap_or_else` of
a `Result<T, !>`, a default made, or a function passed, for nothing; its
fix is `unwrap()`.

## In the library

- `Option::unwrap` and `expect`: `with panic if self.is_none()`;
  `Result::unwrap` and `expect`: `with panic if self.is_err()`.
- `List::push`: `with alloc if self.len() == self.capacity()`, and it
  ensures the capacity doesn't shrink; `with_capacity(n)` gives one with
  capacity `n`. So a list made with room for what's pushed into it
  doesn't allocate:

```rust
use alloc::list::List;

fn fill(l: { l: List<u32> | l.len() == 0 && l.capacity() >= 3 }) -> List<u32> with {} {
    let mut m = l;
    m.push(1);
    m.push(2);
    m.push(3);
    m
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = fill(List::with_capacity(3));
    out.line(&l.len())?; // expect: 3
    l.discard();
    Ok(())
}
```

## Why it's sound

Write a function's effects as some it has always, `U`, and some it has
when a condition holds, `a if P_a`; one it doesn't say has the condition
`false`.

- **Lemma C1 (triggers).** Each site has, for each effect, a *trigger*: a
  formula that's true wherever the site has the effect when it's run. An
  operator's is that it overflows; a call's, of an effect its callee has
  `if Q`, is `Q` of the arguments; any other's is `true`.
- **Lemma C2 (bodies).** For every site `s` of the body, with what's known
  there `Φ`, and every effect `a` it has that isn't in `U`: `Φ ∧ T ⊢
  P_a`, where `T` is its trigger and `P_a` the condition of the
  parameters at the call. For `a` the function doesn't say, that's `Φ ⊢
  ¬T`, which is the operators' rule as it was. A site the walk doesn't
  see, such as a static's use, keeps its effect.
- **Lemma C3 (calls).** A call of `h` has `h`'s `U`, and each `a if Q` of
  `h`'s whose `¬Q`, of the arguments, isn't proved where it is.
- **Lemma C4 (values).** A value of an enum type is one of the variants
  whose fields all have types with values: as nothing builds one of the
  others, by matching's own reasoning.
- **Lemma C5 (the stack).** When an effect `a` happens, every function on
  the stack has `a` always, or `if P_a` with `P_a` true when it was
  called. The innermost, by C2: its site's trigger is true then, as the
  effect happens, and what's known there is true, so `P_a` was. Its
  caller called it at a site whose trigger is the callee's condition, of
  the arguments it gave, which is true by the same argument; so, by C2
  again, the caller's `P_a` was true, and so on down. A function that
  doesn't say `a` would need `false` to hold: it isn't on the stack. As in
  docs/07-effects.md, that's an induction on the stack, which recursion
  doesn't change.
- **Lemma C6 (the call).** The checker says the condition of the
  arguments at the call, and the body of the parameters as they're given,
  before anything in it runs: the same values.
- **Lemma C7 (the oracle).** The interpreter works each condition out
  when a function's called, and lets only those whose conditions held
  happen in it, so each panic and allocation is checked against C5.

## What it doesn't do

- **`count` and `enumerate` still say `with panic`.** An exact iterator's
  count can't overflow, but that depends on the iterator's *type*, not on
  a value a call is given: proving it would need an invariant naming
  `len` inside a body every iterator shares, `count`'s loop or
  `Enumerate`'s fields, which only an exact one has. That's *conditional
  invariants*, invariants that hold where a bound does, which `Skip` and
  `Filter` need for laws too (docs/88-iterator-laws.md).
- **No conditions on trait methods.** An impl's method would have to say
  as much as its trait's, as contracts do (docs/86-trait-contracts.md),
  checked of each.
- **No conditions on effect parameters.** `Option::map` may call its
  function only if it's a `Some`, but the function's effects are the
  caller's to know.
- **A call in a branch that can't be taken still has its effects**, unless
  the walk knows the branch can't be: an operator's is proved, or a
  callee's condition ruled out, but a `panic(..)` is a site whose trigger
  is `true`.

## Decisions

- **D1: conditions on values.** A type-only rule (`E: Uninhabited`) would
  have done `unwrap` of a `Result<T, !>` and nothing else; overloading
  `unwrap` on `Result<T, !>` would make what a call does depend on how
  much the caller knows of a type. A predicate of the arguments, which
  the refinement checker already proves things of, does both, and more.
- **D2: `with panic if P`**, with no new keyword, read as "with panic, if
  P". `unless` would say what callers prove; `when` would be a word for
  the same thing as `if`.
- **D3: `count` and `enumerate` wait** for conditional invariants.
- **D4: `into_ok` is removed**, as `unwrap` does what it did.

## Exercises

1. Write `fn div(a: u32, b: u32) -> u32 with panic if b == 0`. Why can't
   it be `with panic if b == 0 || a == 0`, though that's true too? Is it?
2. `unwrap` calls `expect`. What must `expect`'s condition be for
   `unwrap`'s body to check?
3. Why is the condition of what's behind a `&mut` what was there at the
   call? Write a function that would go wrong if it were what's there at
   return.
4. Make `List::insert` say when it allocates. What must it ensure of the
   capacity?
5. In `Walk::cut_effects`, rule out a callee's conditions without proving
   anything. Which tests fail, and does the interpreter notice?

## Reading

- Nielson and Nielson, *Type and Effect Systems* (1999), on effects that
  depend on values.
- Xi and Pfenning, *Dependent Types in Practical Programming* (1999), and
  DML's array bounds: an operation that can't fail where an index is
  proved in range.
- Vazou, Seidel, Jhala et al., *Refinement Types for Haskell* (2014), on
  partial functions made total by refinements.
- Rust's `Result<T, Infallible>` and the `!` type, and its
  `into_ok`, a method for what a condition says here.
