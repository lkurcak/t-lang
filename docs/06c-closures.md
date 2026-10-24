# Step 6c: closures as `Fn` traits

Code: `src/check/closures.rs` (closure types, how a closure captures, and
which `Fn` traits it implements), `src/traits.rs` (`ClosureDef`, `FN_TRAITS`
and the built-in impls), `src/borrowck.rs` (`closure`: checking a closure's
body) and `src/borrowck/build.rs` (making a closure, and calling one),
`src/lift.rs` and `src/interp.rs` (`Closure`).

```rust
fn apply<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(x) }

fn count<effect e, F: FnMut() with e + Discard>(mut tick: F) with e {
    tick();
    tick();
}

fn main() with panic {
    let k = 10;
    print(apply(|x| x * k, 4)); // expect: 40
    let mut n: i64 = 0;
    count(|| n = n + 1);
    print(n); // expect: 2
}
```

Until now a closure that captured was a value of a function type, holding
copies of what it captured, and it could only be passed down, never returned
or stored (docs/03-inference.md). That kept its variables safe on the stack,
but it couldn't change them, and it couldn't be stored, even in a struct
that doesn't outlive them. This step does what Rust does instead.

## A closure is a struct

`|x| x * k` is a value made of its code and what it captures, `k`. That's a
struct, with a field for `k`, and a function that takes it:

```text
struct Closure<'a> { k: &'a i64 }
fn call(self: &Closure<'a>, x: i64) -> i64 { x * *self.k }
```

Turning a closure into that is *closure conversion*, and here it's done in
the types. Each closure that captures has a type of its own: a struct of what
it captures, generic over the type parameters in scope where it's written,
with a lifetime for each region in its fields' types and in its signature
(`traits::ClosureDef`). It can't be named, and messages show it as
`{closure}`. Its fields hold a reference to each variable it captures, or
its value, so the borrow checker sees what it borrows, as it does for any
struct: the closure can't outlive `k`.

Calling it is what a trait says. There are three, built in:

- `FnOnce(A) -> R`: calls take the closure, so it can be called once.
- `FnMut(A) -> R`: calls borrow it mutably. `trait FnMut: FnOnce`.
- `Fn(A) -> R`: calls borrow it. `trait Fn: FnMut`.

`F: Fn(i64) -> i64` is a bound, as `T: Show` is, and it's how generic code
takes a closure. What a call takes and gives is an equality bound on an
associated type of `FnOnce` that nothing else can name: `F: Fn(i64) -> i64`
is `F: Fn<(signature) = fn(i64) -> i64>`, worked out and checked as in
docs/06b-associated-types.md. Inside `apply`, `f(x)` is `F`'s call, with
`F`'s signature.

Functions and closures that capture nothing keep their function types, and
implement all three traits, so `apply(inc, 1)` works too. A function type
holds only those, which is why `fn(i64) -> i64` can be a field or a result
type while a closure that captures can't be put there:

```rust,error
fn adder(n: i64) -> fn(i64) -> i64 {
    |x| x + n // error: expected `fn(i64) -> i64`, found a closure that captures `n`
}
```

That's what keeps function types sound under the rule trait resolution
relies on: a function value works at every lifetime (`traits::matches`),
which is true when it holds no references. Returning a closure that captures
needs a way to say "some type that implements `Fn`" in a signature, `impl
Fn(i64) -> i64`: that's step 6c.1, docs/06c.1-opaque-types.md.

## How a closure captures

A variable is captured the least way that does what the body does with it,
worked out once the types of the function it's in are known, so that it's
known what's `Copy` (`Checker::settle_closures`):

- by `&` if the body only reads it, or copies it,
- by `&mut` if it changes it or borrows it mutably,
- by value if it moves it.

A `&mut` the body changes things through, as in `|| r.push(1)` or `|| *r =
1`, is borrowed *uniquely*: by `&mut`, but without needing `r` to be `mut`,
as a reborrow `&mut *r` anywhere doesn't (`closure_reborrow.t`).

`move |x| ..` captures everything by value. Then the closure owns what it
captured, and can outlive the variables, but not what they borrow. A whole
variable is captured, not the fields the body uses: `|| p.x` borrows all of
`p` (`closure_capture_modes.t`).

Which trait a closure implements follows from what its body does:

- `FnOnce` only, if it moves out of what it owns,
- `FnMut`, if it changes what it captured,
- `Fn` otherwise.

```rust,error
fn twice<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with {panic, e} { f() + f() }

fn main() {
    let mut n = 0;
    let next = || {
        n = n + 1;
        n
    };
    print(twice(next)); // error: this closure is only `FnMut`, as it changes `n`
}
```

## Checking a closure's body

The borrow checker checks a closure's body as the function its impl's call
is: it takes the closure by `&` for `Fn`, by `&mut` for `FnMut`, or by value
for `FnOnce`, and what it captured are that parameter's fields. A variable
`k` it borrows is `*(*self).k` in there, and one it owns is `(*self).k`.
So a closure that changes `n` through `&self` would be changing it behind a
`&`, which the borrow checker rejects: a closure's body can't do more than
its kind says, however the kind was worked out.

The lifetime of that `&self` is one that only the call knows. A closure
can't give out anything borrowed through it:

```rust,error
fn main() {
    let mut x: i64 = 0;
    let mut c = || &mut x; // error: this closure returns a reference to what it captured
    let a = c();
    let b = c();
    *a = 1;
    *b = 2;
}
```

Each call would hand out a `&mut x`, and two would be alive at once. Through
a `&`, though, what a reference it holds points to stays put for as long as
that reference says, so `|| &n` can return its `&n` for as long as it
borrows `n` (`closure_return_ref.t`). That's the rule for any reborrow: going
through a `&mut` limits it to that reference's lifetime, and going through a
`&` doesn't (Rust's *supporting prefixes*, `Builder::borrow`).

What the body needs of the closure's lifetimes, and of the lifetimes of the
function it's in, it can't decide, so they go up to that function, as
constraints at the closure, as they did in step 5 for a closure's
signature.

A closure's type is invariant in all its parameters. Its fields could say
how each lifetime varies, but its signature uses them too, the other way
round perhaps: `|| &n` returns what it borrows. A covariant closure type
could be made to promise its result lives for `'static`
(`err_soundness_closure_sig_variance.t`).

## Values that must be used up

A closure is `Discard` if everything it captured is, and `Copy` if
everything is `Copy`: a `&mut` isn't, so a closure holding one isn't either.
Closures have no `#[derive]` to write, as their types can't be named, but
nothing here is hidden: their fields are what the function they're in
captured.

Every `Fn` is `FnOnce` too, and a call that takes a closure that only needs
borrowing drops it after. So a closure that owns a value that isn't
`Discard` is `FnOnce` only, and its body must use the value up, whatever it
does with it:

```rust,error
struct Fd { n: i64 }

fn main() {
    let fd = Fd { n: 3 };
    let c = move || fd.n; // error: `fd` is never used up
    print(c());
}
```

Then every closure that implements `FnMut` may be dropped. That's a lemma,
not a supertrait: generic code that drops an `F: FnMut` says `F: Discard`,
as `apply` above does, and could stop saying so if `FnMut` ever got
implementations that aren't closures.

## References to closures

`&F` is `Fn` when `F` is, and `&mut F` is `FnMut` when `F` is, so a closure
can be lent to a function and called again after:

```rust
fn apply<effect e, F: Fn(i64) -> i64 with e>(f: &F, x: i64) -> i64 with e { f(x) }

fn main() with panic {
    let k = 3;
    let add = |x| x + k;
    print(apply(&add, 1) + apply(&add, 2)); // expect: 9
}
```

`f(x)` on a `&F` calls `F` through it, as a method call would look through a
reference to its receiver.

## Running it

`lift` writes each call of what isn't a function as a call through a
borrow, as the borrow checker sees it: `inc()` for an `FnMut` `inc` is
`(&mut inc)()`, which, now that `&mut F` is `FnMut`, is a T program too
(`t mono` shows it). The interpreter keeps a closure as its code and its
fields, and in its body, binds each captured variable to where it is: the
variable a field points to, or the field (`Interp::call_closure`). So
reading a dangling one would be caught as any dangling reference is.

Monomorphisation used to check its output again, to find closures escaping
through generic code. Closure types say what they borrow wherever they go,
so there's nothing left to find, and it no longer does. A copy of a generic
function can't go wrong where the function didn't.

## Design decisions worth noticing

- **The `Fn` traits are built in.** T's traits have no type parameters, so
  `Fn(i64) -> i64` can't be a trait applied to `i64`; instead, the signature
  is an equality bound on an associated type only these bounds can name.
  Nothing else can implement them, so there's no coherence question.
- **Capture by the least that works.** As in Rust, so closures borrow by
  default and `move` is the exception.
- **Whole variables, not paths.** Capturing `p.x` rather than `p` would
  accept more programs, and can be done later without breaking any.
- **Function types stay.** Only closures that capture get types of their
  own; a function type is still a code address, and a closure capturing
  nothing still fits in one.
- **Invariant closure types.** Simple, and sound whatever the signature
  does; it rejects little in practice, as a closure's type is never written.
- **No supertrait `FnMut: Discard`.** Adding it later breaks nothing, and
  removing it would.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| returning closures, `-> impl Fn(i64) -> i64` | existential types, and which lifetimes they capture | 6c.1 |
| a closure's type generic over lifetimes, `\|x: &i64\| -> &i64 { x }` called at two | higher-ranked closure types | 6e.1 |
| `dyn Fn(i64) -> i64` | no trait objects | — |
| `Box<F>` callable | only useful with `dyn` | — |
| capturing fields, `p.x`, not `p` | path rules; relaxes compatibly | — |
| the result of `F` as `F::Output` | the bound always says it | — |
| implementing `Fn` for a struct | needs a call method taking any arguments | — |

## Exercises

1. Write the struct and the function that `|y| x + y` turns into, for
   `x: i64` captured by `&`. Then for `move |y| x + y`.
2. Why must a closure that changes a captured `n` be called through `&mut`?
   Write the program that would go wrong if `Fn` closures could change
   what they capture.
3. Make the closure type covariant in the lifetimes of its fields only, and
   run `err_soundness_closure_sig_variance.t`. What happens, and why?
4. `|| &mut x` is rejected, but `|| &x` isn't. Which part of
   `Builder::borrow` makes the difference?
5. A closure owning a linear value is only `FnOnce`. Suppose it were `Fn`
   instead, and never `Discard`. What could a program do with it? What
   couldn't it?
6. Capturing by path: change `uses_in` to capture `p.x` rather than `p`
   for `|| p.x`. Which programs does it accept that weren't? What should
   `|| (*r).x` capture, for `r: &mut P`?
7. Rust's closures are `Copy` when their captures are, as here. Write a
   program where that matters.

## Reading

- *The Rust Reference*, the chapters on closure expressions and closure
  types.
- RFC 114 (unboxed closures), RFC 2132 (`Copy` and `Clone` for closures),
  and RFC 2229 (capturing paths, not variables).
- Andrew Appel, *Compiling with Continuations*, chapter 10: closure
  conversion.
- The rustc dev guide, *Closure capture inference* and *Region inference*
  (closure region requirements).
