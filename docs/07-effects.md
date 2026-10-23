# Step 7: effects

Code: `src/check/effects.rs` (sites, the `⊆` that wait for the end of a
function, and solving them), `src/check/expr.rs` and
`src/check/paths.rs` (where operators, calls and `Box::new` have effects), `src/check/unify.rs` (effect sets in
unification, and `Fn` bounds as upper bounds), `src/traits.rs` (`fits`,
`impl_args`), `src/check/traits/impls.rs` (an impl's methods against
its trait's), `core/panic.t`, `core/num.t`, and `src/interp.rs`
(`Interp::effect`, which checks it all at run time).

```rust
fn max(a: i64, b: i64) -> i64 {
    if a < b { b } else { a }
}

fn mean(a: i64, b: i64) -> i64 with panic { (a + b) / 2 }

fn twice<e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(f(x)) }

fn main() with panic {
    print(max(3, 4));                // expect: 4
    print(mean(3, 5));               // expect: 4
    print(twice(|x| max(x, 10), 1)); // expect: 10
    print(twice(|x| x * 3, 1));      // expect: 9
}
```

A function's type now says what calling it may do besides give its
result: its *effects*. There are two (step 9 adds a third, `unsafe`: see
docs/09-unsafe.md). `panic` says a call may abort the
program, and `alloc` that it may allocate. `max` has neither, and that's
checked: nothing it does, and nothing it calls, can panic. `mean` adds,
and `+` panics when the sum doesn't fit in an `i64`, so `mean` says `with
panic`. A `main` with no `with` is a whole program that can't panic.

This is what the README promised from the start, "errors are values, not
exceptions", carried through: a panic is still possible, but never
hidden, and a function that can't fail says so in a way the compiler
checks. Rust can't say it at all: any function may panic, and finding out
which do means reading every function they call.

## What has effects

- **Arithmetic.** `+`, `-`, `*` and unary `-` panic on overflow, and `/`
  and `%` on dividing by zero, or the least value of a signed type by
  `-1`; since step 11, `as` too, when the number doesn't fit in the type
  it's converted to (docs/11-integers.md). Not where the checker proves
  they can't (step 8, docs/08-refinement-types.md): not `-5`, nor `/` or
  `%` by a literal other than `0` or `-1`, nor `1 + 2`.
- **`panic("why")`**, from the core library, whose type is `fn(&str) -> !
  with panic`: it aborts, saying why (docs/13-strings.md).
- **`Box::new`**, `with {alloc, panic}`: it allocates, and aborts if memory
  has run out. A box that can fail without aborting, `Box::try_new`, comes
  with the `alloc` library.
- **Calls**, which have what the callee's type says.

`core::num` has arithmetic that can't panic: `checked_add`, `checked_sub`,
`checked_mul`, `checked_div`, `checked_rem` and `checked_neg`, each giving
`None` where the operator panics. The operators are what they are with a
`panic` on `None`:

```rust
fn add(a: i64, b: i64) -> i64 with panic {
    match checked_add(a, b) {
        Option::Some(c) => c,
        Option::None => panic("no value"),
    }
}

fn main() with panic {
    print(add(1, 2));                                             // expect: 3
    print_bool(checked_add(9223372036854775807i64, 1).is_none()); // expect: true
}
```

`Range::next` adds 1 only to what's less than the range's end, which
can't overflow, so it can't panic: counting through a range, `for i in
0..n`, is as safe as the body. In step 7 the checker couldn't see that,
and it used `checked_add`; since step 8 it proves it, as it proves that
`checked_add`, `checked_sub` and `checked_div` don't panic, which are
written in T with the operators now (see
docs/08-refinement-types.md). Only `checked_mul`, which needs to reason
about products of unknowns, is an intrinsic. Since step 11 each is
generic, for every integer type (docs/11-integers.md).

## Saying them

Effects are written after the result, before any `where`: `with panic`,
or a set, `with {alloc, panic}`. Without a `with`, there are none. Every
function says its own, as it says its types: they're never worked out
from its body, so changing a body can't quietly change what its callers
can count on. A body with an effect its signature doesn't have is an
error, with a fix that adds it:

```rust,error
fn next(n: i64) -> i64 { n + 1 }

fn main() {
    print(next(1));
}
// error: `+` may panic, but `next` doesn't say `with panic`
```

Saying an effect a function can't have is a warning (`unused_effects`),
whose fix takes it out, but only where nothing outside can be counting on
it. A `pub` function may say `with panic` to keep the right to panic
later, without breaking its callers when it does. A trait's method says
what its impls may do, and an impl's method is public wherever the trait
is.

## Closures and effect parameters

A closure has no signature to say its effects in, so they're worked out
from its body, as its captures are. A bound says what a closure, or
anything else called through a type parameter, may do, *at most*: `F:
Fn(i64) -> i64` holds of what can't panic, and `F: Fn(i64) -> i64 with
panic` of what may or mayn't.

What a higher-order function does depends on what it's given, so it's
generic in that: `e` in `twice` is an *effect parameter*, named in
lowercase among the type parameters, as types are named with a capital.
`twice` has the effects `e`, and `F` has at most `e`. Each call picks `e`,
as it picks a type parameter, but it's never written: it's the *least* set
that makes the bounds hold, the effects of the closure passed. So
`twice(|x| max(x, 10), 1)` can't panic, and `twice(|x| x * 3, 1)` may.

Sets of effects, with union, form a *join-semilattice*, so the least set
always exists. An effect parameter is a type of a kind of its own, as in
Koka: it's instantiated, substituted and monomorphised as any type
parameter is, and an effect variable is a type variable. What's new is
`⊆`: a closure's effects must contain its body's, and an `Fn` bound's
effects must contain the closure's. Those wait until the end of the
function, when each variable left gets the least set that makes them all
hold (`Checker::settle_effects`).

## Function types

A function type says its effects too: `fn(i64) -> i64 with panic`. A
function that can't panic will do where one that may is wanted, so a
function's type gains effects where it's used as a whole value:

```rust
fn inc(n: i64) -> i64 with panic { n + 1 }

fn id(n: i64) -> i64 { n }

fn apply(f: fn(i64) -> i64 with panic, x: i64) -> i64 with panic { f(x) }

fn main() with panic {
    print(apply(inc, 1)); // expect: 2
    print(apply(id, 4));  // expect: 4
}
```

Never the other way: a `fn(i64) -> i64` can't hold `inc`. Nor inside
another type: there's no subtyping, so an `Option<fn()>` isn't an
`Option<fn() with panic>` (`err_effect_coerce_nested.t`). That can come
later without breaking anything.

## Traits

A trait's method says its effects, and an impl's can't have more:
generic code calling it knows only the trait's. So `Eq::eq`, and with it
`==`, can't panic.

What effects an iterator's `next` has depends on the iterator, as the
type it gives does, so `Iterator` has an *associated effect*, which each
impl says, as it says `Item`:

```rust
#[derive(Discard)]
struct Down { n: i64 }

impl Iterator for Down {
    type Item = i64;
    effect e = panic;

    fn next(&mut self) -> Option<i64> with panic {
        if self.n == 0 {
            return Option::None;
        }
        self.n = self.n - 1;
        Option::Some(self.n)
    }
}

fn first<I: Iterator<Item = i64> + Discard>(mut it: I) -> Option<i64> with I::e { it.next() }

fn main() with panic {
    print(first(Down { n: 3 }).unwrap()); // expect: 2
    print(first(0..3).unwrap());          // expect: 0
}
```

`I::e` is a projection, as `I::Item` is, and normalises the same way:
`first(0..3)` has the effects of `<Range<i64> as Iterator>::e`, which are
none. An adapter has those of what it adapts and what it calls: `Map`'s
are `{I::e, f}`, where `f` is the impl's effect parameter for its
closure's. That isn't in the impl's type, as a type parameter must be
unless a bound decides it (docs/07.1-impl-parameters.md), but it doesn't
need to be: it's the least set that makes the impl's
bound `F: FnMut(I::Item) -> U with f` hold, which depends only on `F`
(`traits::impl_args`). So the impl still means one thing for each type it's
for, which is what coherence needs.

## Why it's sound

> A function whose effects don't have `panic` never reaches a panic.

Look at the stack when a panic happens, at an operator, `panic` or
`Box::new`. The function running it has `panic` among its effects, as its
body's sites must be among them. Its caller called something whose
effects have `panic`, at the effect arguments of that call: directly, or
through a closure, whose effects are its body's and at most its bound's,
or a function value, whose type gains effects but never loses them, or a
trait's method, whose impl has at most the trait's. So the caller has
`panic` too, and so on down. The stack is finite, so that's an induction,
and it says every function on it has `panic`: none that doesn't can be
there. `alloc` is the same.

That it's induction on the stack, not on how long the program runs, is
why recursion needs nothing more: assuming a recursive call has the
effects the function says is fine, as no effect is *proved* to not
happen by a call that never returns. Monomorphising puts each effect
argument in, which keeps all this.

The interpreter checks it as it goes: at each panic and allocation, every
function on the stack must say it, or it stops with "type soundness
violated", as it does on a leak.

## What isn't an effect yet

**Divergence.** "Can't panic" isn't "finishes": `fn spin() -> ! { loop {}
}` has no effects. Saying a function finishes is a different kind of
claim, and the induction above doesn't prove it. "No recursion" isn't
enough either. With `struct R { f: fn(&R) -> i64 }` and `fn app(r: &R) ->
i64 { (r.f)(r) }`, the call `app(&R { f: app })` never ends, and nothing
in it calls itself: that's *Landin's knot*. And a trait's method can call
a generic function that calls the trait's method again, which only
monomorphising shows. Tracking it soundly takes a termination checker,
which is what a function being *total* needs too.

**Running out of stack**, which aborts, untracked. A program with no
recursion has a bound on how deep its calls go, which a compiler can
work out; that's for when there's a backend.

## Design decisions worth noticing

- **Effects are written in every signature**, `main`'s too, rather than
  inferred: like types, they're what callers count on.
- **Overflow is a panic, tracked**, rather than wrapping silently. That
  puts `with panic` on most arithmetic, which is honest: it can panic.
  Refinement types (step 8) will prove when it can't.
- **Running out of memory is a panic**, so a function without `panic`
  never aborts, but for the stack.
- **Bounds are upper bounds, and effect parameters take their least
  solution**, so a call never has more effects than what it's given does.
- **Lowercase means an effect parameter.** A type parameter named `t` is
  an effect parameter now. Case already said what a name is.
- **No subtyping.** A function type gains effects only where a whole
  value is used.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| divergence as an effect, total functions | needs a termination checker | — |
| `expect`, a message for `panic` | needs strings | 13 ✅ |
| `Box::try_new` | fallible allocation belongs with `alloc` | `alloc` |
| bounds on associated effects, `I: Iterator<e = {}>` | not needed yet | — |
| effect arguments in turbofish, other than `_` | not needed yet | — |
| subtyping of function types inside other types | coercion where a whole value is used does | — |
| catching a panic | would need more effects, as handlers | — |

## Exercises

1. Write `fn sum(n: i64) -> Option<i64>`, the sum of `0..n`, so that it
   can't panic. What does it give when the sum doesn't fit?
2. Why must `Eq::eq` be unable to panic? What would `==` on an
   `Option<T>` need otherwise?
3. In `Checker::settle_effects`, give every effect variable all of
   `{alloc, panic}` instead of its least solution. Is that still sound?
   Which tests fail, and why?
4. `Filter`'s impl has an effect parameter `p` that isn't in its type.
   Write an impl whose effect parameter nothing decides, and read the
   error. Why would accepting it break coherence?
5. Make `fn app(r: &R) -> i64` from *What isn't an effect yet* loop
   forever. Which of T's features does Landin's knot need, and which
   would a termination checker have to restrict?

## Reading

- Daan Leijen, *Koka: Programming with Row Polymorphic Effect Types*
  (2014): effects as types of their own kind, inferred, with divergence
  as an effect and why it needs care.
- Lucassen and Gifford, *Polymorphic Effect Systems* (1988), where effect
  parameters start.
- Nielson and Nielson, *Type and Effect Systems* (1999), for the general
  shape, and subeffecting.
- Lindley, McBride and McLaughlin, *Do Be Do Be Do* (Frank, 2017), on
  effect polymorphism without writing it everywhere.
- Rust's *keyword generics* initiative, on being generic over `const` and
  `async`, the same problem from the other end.
- Peter J. Landin, *The Mechanical Evaluation of Expressions* (1964), and
  the knot that bears his name.
