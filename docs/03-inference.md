# Step 3: type inference and closures

Code: `src/check/unify.rs` (`Checker::unify`), `src/check/mod.rs`
(`Checker::lambda` and `Checker::check`) and `src/interp.rs` (`Closure`).

```rust
fn adder(n: int) -> fn(int) -> int { |x| x + n }

fn main() {
    let twice = |f, x| f(f(x));
    print(twice(adder(5), 0));   // 10
}
```

Nothing says what `f` and `x` are, yet the checker works out that `twice` is
`fn(fn(int) -> int, int) -> int`. This step is about how.

## Closures: the other half of functions

Step 1 had function *elimination*, T-App (calling a function), but the only
way to *introduce* one was a top-level `fn`. A closure is the lambda calculus's
λ, written inline:

```text
     Γ, x1 : T1, .., xn : Tn ⊢ e : R
  ------------------------------------------ (T-Abs)
   Γ ⊢ |x1, .., xn| e : fn(T1, .., Tn) -> R
```

The body is checked in Γ extended with the parameters, so it can use any local
in scope: `|x| x + n` reads the `n` around it. That's what makes it a
*closure*: it closes over its environment. A top-level function only ever sees
its parameters and other top-level functions.

Closures and top-level functions have the same types, so either can go
wherever a `fn(int) -> int` is wanted (`tests/t/closures.t`).

## Where do T1..Tn come from?

T-Abs has a problem the step 1 rules didn't: nothing in `|x| x + 1` says what
T1 is. Rules are read bottom-up as an algorithm, and this one needs to *guess*.
There are three places the checker can learn it from, tried in this order:

1. **An annotation:** `|x: int| x + 1`.
2. **The context:** in `apply(|x| x + 1, 2)`, `apply` takes a
   `fn(int) -> int`, so `x` must be an `int`. That's bidirectional typing,
   below.
3. **How `x` is used:** `x + 1` only works if `x` is an `int`. That's
   inference by unification, next.

## Type variables and unification

When there's no annotation and no context, `x` gets a **type variable**, a
placeholder for a type not known yet (`Type::Var`; error messages show it as
`_`). Checking carries on as usual, and every rule that would have compared two
types now *unifies* them instead: it makes them equal by solving variables.

`Checker::unify` is Robinson's algorithm (1965), in three cases:

- a variable unifies with any type, by being solved as that type,
- two function types unify if their parameters and results do,
- anything else must be the very same type.

A solution holds for the rest of the function, so the checker learns about
types as it goes. Here's `twice` again:

```text
|f, x| f(f(x))        f : α,  x : β               (fresh variables)

f(x)       f is called, so α must be a function:  α = fn(γ) -> δ
           x is its argument:                     γ = β
f(f(x))    f(x) : δ is its argument:              δ = β
           result: δ, which is β

twice : fn(fn(β) -> β, β) -> β

twice(adder(5), 0)
           fn(int) -> int = fn(β) -> β:           β = int
           0 : int = β                            fine, β is int
```

So `twice(adder(5), 0)` is an `int`. Notice that `β` was only found at the
call, two lines after `twice` was defined. Unification doesn't care what
order it learns things in.

Every variable must be solved by the end of the function. If one isn't, as
in `let f = |x| x;` with `f` never called, that's an error asking for an
annotation (`err_infer_unknown.t`). The interpreter wouldn't mind, since it
never looks at types, but a compiler needs to know how big a value is and how
to pass it.

### The occurs check

There's one more condition in the first case: a variable can't be solved as a
type that contains it. Try typing self-application:

```text
|f| f(f)       f : α
               f is called with f:   α = fn(α) -> β
```

No finite type is equal to a function type that takes *itself*: substituting
gives `fn(fn(fn(...) -> β) -> β) -> β` forever. So unify checks whether `α`
*occurs* in `fn(α) -> β` first, and reports an infinite type
(`err_infer_infinite.t`).

That's not a technicality. `|f| f(f)` is the heart of the untyped lambda
calculus's paradoxes: `(|f| f(f))(|f| f(f))` runs forever, and Curry's
Y combinator is built from it. The simply typed lambda calculus rejects it,
and that's why every STLC program halts (T only has unbounded loops through
top-level recursion). Step 5 gives `α = fn(α) -> β` a solution after all, as
a *recursive type*.

## Bidirectional typing: when order matters

Unification is order-independent only for constraints of the form "these two
types are equal". Some rules need to *decide* something with a type, and
need it known right then:

```rust
struct P { x: int, y: int }
fn on_point(f: fn(P) -> int) -> int { f(P { x: 3, y: 4 }) }

on_point(|p| p.x)       // fine
let get = |p| p.x;      // error: the type of this must be known
```

T-Field asks *which struct* `p` is, to find its field `x`. A variable is no
answer, and "some struct with a field `x`" isn't a type.

The fix is to push types *down* as well as up. The checker has two modes,
already there since step 1:

- `synth(e)` computes `e`'s type from `e` alone (Γ ⊢ e ⇒ T),
- `check(e, T)` is told the type `e` should have (Γ ⊢ e ⇐ T).

Until now, checking was just "synthesize, then compare". It becomes its own
mode for closures: `check(|p| p.x, fn(P) -> int)` gives `p` the type `P`
*before* looking at the body. Every place with a known type checks against it:
arguments against parameter types, `let x: T = e`, a function's body against
its return type, struct fields, and blocks and `match` arms against the type
wanted for the whole. So in practice closures passed to, stored in or returned
from something typed hardly ever need annotations.

This is how Rust works too (`let get = |p| p.x;` is error E0282 there), and
Scala, Swift and TypeScript. The pure alternative, Hindley–Milner as in
OCaml and Haskell, needs every constraint to be an equation, so it fixes the
problem in the language instead: an OCaml record field name belongs to
exactly one record type.

### Checks that wait

`|a, b| a == b` is the other case. T-Eq needs to know that `a`'s type contains
no function, but nothing needs to be *decided* from the answer, so the check
can wait: `Checker::comparisons` collects it and `solve_all` checks it once the
function's types are all known (`err_infer_compare_fns.t`). A constraint that
isn't an equation, carried along until types are known: that's a small
version of what step 6 does with traits (`T: Eq`).

## Where inference stops

Top-level functions still need full signatures. Each function body is its own
inference problem: its variables are made, solved and checked off inside it,
so a function's type never depends on what its callers do. That keeps errors
local and signatures readable, and it's Rust's choice too. Haskell and OCaml
infer top-level types as well.

And a variable has *one* type:

```rust
let id = |x| x;
print(id(1));            // id : fn(int) -> int from here on
print_bool(id(true));    // error: expected `int`, found `bool`
```

In Hindley–Milner, `let` would *generalize* `id` to `∀α. fn(α) -> α`, a type
that works for every `α`, and each use would get its own copy. That's
parametric polymorphism, which is step 4.

## Closures and memory

A closure is a value made of two things: its code, and the values of the
locals it uses, its **environment**. `adder(5)` returns `|x| x + n` with
`n = 5` packed alongside. Turning a closure into a (code, environment) pair is
called **closure conversion**, and it's how every compiler does it. C has no
closures, so C APIs do it by hand: a function pointer plus a `void *context`.

The interpreter copies the locals the closure uses into it when it's created
(`Value::Closure`). Those are its **free variables**: the names its body uses
but doesn't bind itself, worked out once by the parser (`Lambda::captures`).
Since nothing in T can change a variable once it's bound, a copy can never
disagree with the original, so copying is the same as sharing. The order
still shows (`closures.t`):

```rust
let n = 1;
let get = || n;
let n = 2;          // a new `n`, shadowing the old one
get() + n           // 3
```

The hard part is where the environment *lives*. `adder`'s `n` is a parameter,
on `adder`'s stack frame, but the closure outlives that frame: it's returned.
This is the classic **upward funarg problem**. Closures that only go *down*
(passed to functions, never returned or stored) can point into the stack, as
Pascal's nested procedures do. Ones that go up need the environment somewhere
that outlives the call: the heap. Then someone has to free it, and copying a
closure copies a pointer to its environment, so who's that? A garbage
collector would answer for most languages. T has none, so the answer is
ownership, step 7. Until there's a real backend, the interpreter gets to not
care.

Rust's answer, for comparison: every closure has its own unnamed type,
exactly the size of its environment, so it can live on the stack and be
moved like any other value. `fn(int) -> int` there is only a plain function
pointer, which capturing closures don't fit. Talking about "some closure
type" takes generics (step 4) and traits (`impl Fn(int) -> int`, step 6). T's
`fn(int) -> int` is more like Rust's `Rc<dyn Fn(i64) -> i64>`: any function,
behind a pointer.

## Design decisions worth noticing

- **Rust's syntax:** `|x| x + 1`, `|x: int| -> int { .. }` (a return type
  needs a block), and `|| 1` with no parameters. `||` is a single token, so
  the parser accepts it where an expression starts as an empty parameter list.
- **Only free variables are captured**, and copies of a closure share one
  environment. Capturing every local in scope would mean the same thing, but
  a closure would copy every closure before it, each holding every closure
  before *that*: twice the size at each step (`closure_chain.t`).
- **Unsolved variables are errors**, not defaulted to some type. Rust does
  the same, except for number literals, which default to `i32`.
- **Variables never leave a function.** Every global's type is fully known,
  so `solve_all` at the end of each function can demand that all of that
  function's variables are solved.
- **The context isn't required.** `check` uses the expected type when it's
  there and falls back on unification when it isn't. *Local type
  inference* (Pierce and Turner, 2000) uses only the context, with no
  unification, which is simpler and predictable but needs more annotations:
  `let inc = |x| x + 1;` would be an error.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `let id = \|x\| x;` usable at two types | let-polymorphism is parametric polymorphism | 4 |
| a closure calling itself | its name isn't in scope in its own body; use a top-level `fn` | — |
| changing a captured variable | mutation, and who owns the environment | 7 |
| patterns as parameters, `\|P { x, y }\| ..` | easy, just not needed yet | — |
| inferring top-level signatures | on purpose, see *Where inference stops* | — |

## Exercises

1. Delete the occurs check in `Checker::unify` and run `|f| f(f)`. What
   happens, and where?
2. Infer the type of `|f, g, x| f(g(x))` by hand, as in the `twice` example.
   Which variables does it leave unsolved?
3. Make `let get = |p| p.x;` work when exactly one struct has a field `x`.
   What should happen when two do?
4. Make `let` polymorphic: when a `let` binds a closure, replace the
   variables that appear nowhere in Γ with type parameters, and give each use
   of the variable fresh copies of them. Then `id(1)` and `id(true)` both
   check. This is *Algorithm W* (Damas and Milner, 1982), and step 4 in
   miniature.
5. `Lambda::captures` lists top-level functions a closure calls too, and
   the interpreter skips them. Why can't it tell them apart from locals
   when it computes them? Could the checker?
6. In T-App, when `f`'s type is a variable, `Checker::synth` unifies it with
   a function type of fresh variables, and claims that can't fail. Why not?

## Reading

- *Types and Programming Languages* (Pierce), chapter 22 (type
  reconstruction: constraints and unification).
- Luis Damas and Robin Milner, *Principal type-schemes for functional
  programs*, POPL 1982. The original Hindley–Milner inference.
- Jana Dunfield and Neel Krishnaswami, *Bidirectional Typing*, ACM Computing
  Surveys, 2021. A survey of the checking/synthesis split.
- Benjamin Pierce and David Turner, *Local Type Inference*, TOPLAS 2000.
- *The Rust Reference*, chapter on closure expressions, and the rustc dev
  guide's chapter on type inference.
