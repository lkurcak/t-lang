# Step 3: type inference and closures

Code: `src/check/unify.rs` (`Checker::unify`), `src/check.rs`
(`Checker::lambda` and `Checker::check`), `src/interp.rs` (`Closure`), and
for nested functions `src/check/scope.rs` and `src/lift.rs`.

```rust
fn main() {
    let n = 5;
    let twice = |f, x| f(f(x));
    print(twice(|x| x + n, 0)); // expect: 10
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
*closure*: it closes over its environment. A `fn` only ever sees its
parameters and other functions.

Closures and functions have the same types, so either can go wherever a
`fn(int) -> int` is wanted (`tests/t/closures.t`).

### Nested functions

A `fn` can also be declared in a block, as a statement. It means exactly what
it would at the top level; only its name is scoped, to the block
(`tests/t/nested_fns.t`):

```rust
fn main() {
    print(fact(5)); // expect: 120

    fn fact(n: int) -> int {
        if n == 0 { 1 } else { n * fact(n - 1) }
    }
}
```

The name is in scope in the *whole* block, before the `fn` as well as after,
so a nested `fn` can call itself, and nested `fn`s in the same block can call
each other, in any order. It can be generic and have a doc comment, like any
`fn`. It shadows any outer function of the same name, and two functions can
each have their own `fn helper`. Like Rust, which has the same rule for
items in blocks, the block's functions shadow the locals bound before the
block, and are shadowed by those bound in it.

What it can't do is capture. It doesn't see the locals around it, or the
type parameters of the function it's in, only other functions and types:

```rust,error
fn main() {
    let n = 5;
    fn add_n(x: int) -> int {
        x + n // error: can't capture `n` in a nested `fn`; use a closure
    }
    print(add_n(1));
}
```

That's what makes it more than a closure with a name. A nested `fn` can be
moved to the top level, or into another function, as it is, and it means the
same. It's a plain function, so it can be returned and stored, as a capturing
closure can't (see *Closures and memory*). And the checker can check it as a
function of its own, with its own type variables, so nothing it does can
change what's inferred around it.

So which to use?

- **A nested `fn`** for a helper that needs no locals: it has a name and a
  signature that says what it takes, it can be recursive or generic, and it
  goes anywhere a function can. Moving it out of the function later is a
  cut and paste.
- **A closure** when it needs the locals around it, or when it's short and
  passed straight to something: `apply(|x| x + 1, 2)`, with its types
  inferred from where it goes.

Since two functions can now have the same name, the checker resolves every
use of a name to its *definition*, through the scopes it's in
(`src/check/scope.rs`), not by looking the name up in one global table. Each
function gets a *symbol* that's unique in the program, like `main::fact`.
Once the program checks, every nested `fn` is moved to the top level under
its symbol (`src/lift.rs`), which is easy since it captures nothing. After
that, nothing needs to know there were nested functions: not
monomorphisation, not the interpreter, and not a backend. `t mono` shows
the symbols; messages and editors show the names as written.

### Local types

A `struct` or `enum` can be declared in a block too, by the same rules: it
means what it would at the top level, its name is in scope in the whole
block, and it shadows any outer type of the same name
(`tests/t/local_types.t`). So a helper type can stay with the nested `fn`s
that use it, inside the one function that needs them:

```rust
fn main() {
    print(perimeter(Rect { w: 2, h: 3 })); // expect: 10

    #[derive(Discard)]
    struct Rect { w: int, h: int }
    fn perimeter(r: Rect) -> int { 2 * (r.w + r.h) }
}
```

Like a nested `fn`, a local type can't use the type parameters of the
function it's in: in `fn wrap<T>`, `struct Wrap { v: T }` is an error, since
at the top level that `T` would mean nothing. Declare `struct Wrap<T>`
instead.

Nothing outside the block can name a local type, but its values can still
leave the block, since inference doesn't need the name:

```rust
fn main() {
    let p = {
        #[derive(Discard)]
        struct P { x: int }
        P { x: 1 }
    };
    print(p.x); // expect: 1
}
```

Types are compared by name, so two local types of the same name are still
two types. They look the same in a message, so when they meet, it says
which is which:

```rust,error
fn f() -> int {
    #[derive(Discard)]
    struct P { x: int }
    fn g(p: P) -> int { p.x }
    fn h() -> int {
        #[derive(Discard)]
        struct P { x: int }
        g(P { x: 1 }) // error: these are two different types called `P`
    }
    h()
}
```

Local types get symbols the same way, like `main::Rect`, and are lifted with
the functions. Finding which type a name refers to is simpler than for a
function, because only another type can shadow a type, never a local
variable, and nothing about it depends on inference. So it's done for the
whole program before checking starts (`src/check/names.rs`), and the
checker looks up each type name by where it's written.

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
top-level recursion). T won't ever solve `α = fn(α) -> β` as it stands: a
type will only be able to hold itself through a named struct or enum, behind
a pointer (step 5), so a name always ties the knot.

## Bidirectional typing: when order matters

Unification is order-independent only for constraints of the form "these two
types are equal". Some rules need to *decide* something with a type, and
need it known right then:

```rust,error
#[derive(Discard)]
struct P { x: int, y: int }
fn on_point(f: fn(P) -> int) -> int { f(P { x: 3, y: 4 }) }

fn main() {
    print(on_point(|p| p.x)); // fine
    let get = |p| p.x;        // error: the type of this must be known
}
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

```rust,error
fn main() {
    let id = |x| x;
    print(id(1));         // id : fn(int) -> int from here on
    print_bool(id(true)); // error: expected `int`, found `bool`
}
```

In Hindley–Milner, `let` would *generalize* `id` to `∀α. fn(α) -> α`, a type
that works for every `α`, and each use would get its own copy. That's
parametric polymorphism, which is step 4.

## Closures and memory

A closure is a value made of two things: its code, and the values of the
locals it uses, its **environment**. In `let add = |x| x + n;`, `add` is the
code of `|x| x + n` with `n` packed alongside. Turning a closure into a (code, environment) pair is
called **closure conversion**, and it's how every compiler does it. C has no
closures, so C APIs do it by hand: a function pointer plus a `void *context`.

The interpreter copies the locals the closure uses into it when it's created
(`Value::Closure`). Those are its **free variables**: the names its body uses
but doesn't bind itself, worked out once by the parser (`Lambda::captures`).
In this step nothing in T can change a variable once it's bound, so a copy
can never disagree with the original, and copying is the same as sharing.
(Step 5 adds mutation, and keeps the copies: a closure has its own, which
it can't change. See docs/05-ownership.md.) The order still shows
(`closures.t`):

```rust,fragment
let n = 1;
let get = || n;
let n = 2; // a new `n`, shadowing the old one
get() + n  // 3
```

The hard part is where the environment *lives*. Take

```rust
fn adder(n: int) -> fn(int) -> int { |x| x + n }
```

`n` is a parameter, on `adder`'s stack frame, but the closure outlives that
frame: it's returned. This is the classic **upward funarg problem**. A
closure that goes up needs its environment somewhere that outlives the call:
the heap. Then someone has to free it, and copying a closure copies a pointer
to its environment, so who's that? Most languages answer with a garbage
collector, or reference counting. T can't: it has to compile to bare metal,
with no runtime.

So in T, **closures that capture are downward-only**: they can be called, and
passed to functions, but not returned or stored in a struct or enum. Then the
environment can simply live in the frame that made the closure, and the
closure can be a pair of pointers, to its code and into that frame, which
only ever reaches frames *above* it on the stack, which are still there.
That's how Pascal's nested procedures work, and Swift's non-escaping closures.
`adder` is an error:

```
type error: cannot return a closure that captures `n`: it keeps `n` on the
stack, and closures that capture can only be passed down, not returned
```

Closures that capture nothing are plain functions, and go anywhere.

The rule (`src/check/escape.rs`) follows *where a value may have come from*,
not its type: `fn(int) -> int` is one type, whether or not the function
behind it captures. A value may be a closure that captures if it's

- a closure literal that captures a local,
- a parameter: the caller could have passed one, or
- a variable bound to one of those, or an `if`, `match` or block that may
  end in one.

Such a value must not be returned (from a function or a closure), or put in a
struct or enum, once its type turns out to be a function type. Call results
and fields are always safe, because they were returned or stored, so they
passed the same check. And since nothing in T can change a variable or reach
the heap, returning and storing are the only ways a value can outlive its
frame. So that's the whole rule. (Step 5 adds two more ways, assigning and
boxing, and the rule covers them too.)

It's also coarse. `fn id(f: fn(int) -> int) -> fn(int) -> int { f }` is an
error, though it only hands the caller back its own closure, which is still
alive. Telling those apart means saying in the type *which* frame a closure
borrows from: `id` returns something that lives as long as its argument. That's
a lifetime, and it's what step 5 (ownership and borrowing) is about. This rule
is the smallest case of one: every function value either lives forever, or
lives as long as "here". (Step 5 gives references lifetimes, but not yet
function values: see docs/05-ownership.md.)

The other way out, for later: give every closure its own unnamed type,
exactly the size of its environment, so it can be returned *by value* and
live wherever its caller puts it. That's what Rust does. `fn(i64) -> i64`
there is only a plain function pointer, which capturing closures don't fit,
and talking about "some closure type" takes generics (step 4) and traits
(`impl Fn(i64) -> i64`, step 6). Each closure having its own type also lets
the compiler inline it into every function it's passed to, so it often
disappears entirely.

The interpreter doesn't care where environments live, and shares them behind
an `Rc`. The rule is in the checker, so that a compiled T never needs to.

## Design decisions worth noticing

- **Rust's syntax:** `|x| x + 1`, `|x: int| -> int { .. }` (a return type
  needs a block), and `|| 1` with no parameters. `||` is a single token, so
  the parser accepts it where an expression starts as an empty parameter list.
- **Capturing closures go down, not up.** See *Closures and memory*.
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
| returning a closure that captures | its environment is on the stack: lifetimes, or a type per closure | 5, 6 |
| a closure calling itself | its name isn't in scope in its own body; use a nested `fn` | — |
| changing a captured variable | mutation, and who owns the environment | 5 |
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
5. `Lambda::captures` lists functions a closure calls too, and the
   interpreter skips them. Why can't it tell them apart from locals
   when it computes them? Could the checker?
6. `fn id(f: fn(int) -> int) -> fn(int) -> int { f }` is rejected, though
   it's harmless on its own. Write a program that would go wrong if it
   weren't, using `id` and a closure that captures. Then: why is it fine for
   `escape_ok.t` to return a field?
7. *Defunctionalization* (Reynolds, 1972) compiles closures without a heap
   differently: every closure of type `fn(int) -> int` in the program
   becomes a variant of one enum holding its environment, and calling one is
   a `match`. Then `adder` works, returning an enum. What's the size of that
   enum? What goes wrong with `closure_chain.t` if each closure there is
   returned?
8. In T-App, when `f`'s type is a variable, `Checker::synth` unifies it with
   a function type of fresh variables, and claims that can't fail. Why not?

## Reading

- *Types and Programming Languages* (Pierce), chapter 22 (type
  reconstruction: constraints and unification).
- Luis Damas and Robin Milner, *Principal type-schemes for functional
  programs*, POPL 1982. The original Hindley–Milner inference.
- Jana Dunfield and Neel Krishnaswami, *Bidirectional Typing*, ACM Computing
  Surveys, 2021. A survey of the checking/synthesis split.
- Benjamin Pierce and David Turner, *Local Type Inference*, TOPLAS 2000.
- John Reynolds, *Definitional Interpreters for Higher-Order Programming
  Languages*, 1972. Where defunctionalization comes from.
- *The Rust Reference*, chapter on closure expressions, and the rustc dev
  guide's chapter on type inference.
