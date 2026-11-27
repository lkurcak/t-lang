# Step 78: callable values

Code: what calling a value gives, `summary::Callable`, kept by the
value's object in `Walk::callables` (`src/check/refine/summary.rs`); a
closure's value made in `Walk::aggregate`, a function's in
`Walk::constant` (`Walk::fn_obj`); what a call through one gives,
`Walk::summary_call`; a function given for `F`, `Walk::fn_summary`; and
both used in `Walk::mir_call` (`src/check/refine/mir.rs`).

Step 74 let a generic call give back what a closure written in it gives
(docs/74): `call(|| five())` is greater than 0. Retrospective 10 found
that this held only for that way of writing it. The same value, written
any other way, came back as any number, on 0.92.0:

```text
fn five() -> { v: u32 | v > 0 } { 5 }

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() with {} {
    let a = call(|| five());
    let _ = 10 / a; // fine
    let f = || five();
    let _ = 10 / call(f); // error: `/` may panic
    let _ = 10 / call(five); // error: `/` may panic
    let _ = 10 / f(); // error: `/` may panic
    let p = five;
    let _ = 10 / p(); // error: `/` may panic
}
```

A program shouldn't need rewriting to keep what it knows. Step 78 makes
what calling a value gives part of the value. All of these are now
greater than 0:

```rust
fn five() -> { v: u32 | v > 0 } { 5 }

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

#[derive(Discard)]
struct Holder<F> { f: F }

fn main() with {} {
    let f = || five();
    let _ = 10 / call(f);
    let g = || five();
    let _ = 10 / g();
    let _ = 10 / (|| five())();
    let _ = 10 / call(five);
    let p = five;
    let _ = 10 / call(p);
    let _ = 10 / p();
    let h = || five();
    let _ = 10 / call(|| h());
    let hold = Holder { f: || five() };
    let _ = 10 / call(hold.f);
}
```

## A closure's value

The refinement checker walks a closure's body where it's made, as every
run of it (lemma S3 of docs/69-loans.md). That walk always checked the
body. Its summary, what's known where the body returns (docs/74), was
thrown away unless the closure was written in a call. Now it's kept with
the closure's *value*, the object of the logic the value is
(`Walk::callables`). A value goes wherever it's moved or copied: into
another variable, a field, a capture of another closure. Paths that meet
keep it if each brought the same value.

Given for a callee's `F`, the value's summary is used as a closure's
written in the call is: it's one more case of what the call's `R` is
(`Cand::Made`, docs/74). Called straight, `f(..)`, what it gives is
what one of its returns gave, with new variables for what the run made
(lemma C1 of docs/74).

A closure made before its call was walked where it was made, not at the
call. So its parameters were any values of their types, not what the call
gives them, as a closure written in the call gets (step 60's D6). That
makes its summary hold whatever the arguments are, and a call that says
what they are can say so of the summary: an integer or `bool` parameter
is the call's argument.

```rust
fn main() with {} {
    let pick = |t: bool| if t { 1u32 } else { 2u32 };
    let one: { v: u32 | v == 1 } = pick(true);
    let half = |w: u32| w / 2;
    let six: { v: u32 | v <= 6 } = half(12);
    let _ = (one, six);
}
```

## A function's value

A function used as a value, `five`, is the same value wherever it's used:
one object of the logic for each function and its type arguments
(`Walk::fn_obj`). It goes wherever it's moved or copied, as a closure's
does, and a closure that gives it back gives back that function, the
same in every run. A call through it, `p()`, is a call of the function: its result
has what the function's signature says, and so does what it was lent:

```rust
use alloc::list::List;

fn push1(l: &mut List<u8>) with alloc ensures l.len() == old(l.len()) + 1 {
    l.push(1);
}

fn below(n: usize) -> { v: usize | v <= n } { n / 2 }

fn main() with alloc {
    let mut l: List<u8> = List::new();
    let p = push1;
    p(&mut l);
    let one: { v: usize | v == 1 } = l.len();
    let b = below;
    let w: { v: usize | v <= 4 } = b(4);
    let _ = (one, w);
}
```

Given for a callee's `F`, a function `g` is what the closure `|xs|
g(xs)`, written in the call, would be: a call of `g` with the values
the callee gives its parameters. So `app(x, half)` knows what
`app(x, |v| half(v))` does:

```rust
fn app<T, R, F: FnOnce(T) -> R + Discard>(x: T, f: F) -> R { f(x) }

fn half(x: u32) -> { r: u32 | r <= x } { x / 2 }

fn main() {
    let x: { v: u32 | v < 10 } = 9;
    let q: { v: u32 | v < 10 } = app(x, half);
    let _ = q;
}
```

That summary comes from walking one call of `g` (`Walk::fn_summary`).
Nothing in it needs checking: `g` was checked on its own, and a function
whose parameters are refined can't be a value (docs/08). What a callee
may write through a `&mut` it gives `F` is still checked where the
function's given (lemma W7 of docs/63): a call of `g` that would fail
gives no summary.

## Where a value isn't known

A value is an object, never a type. Two closures made by the same
closure expression, in two turns of a loop, are two values of the same
type, each with what it captured then. Exercise 2 of docs/74 asked what
keeping summaries by type would break. A closure kept from one turn and
called in the next is one the walk knows nothing of: the loop's head
forgets what the loop changes.

```rust,error
fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main() with panic {
    let mut kept = None;
    for n in 0..9usize {
        if let Some(g) = kept {
            let x: { v: usize | v == n } = call(g);
            let _ = x;
        }
        let f = move || n;
        kept = Some(f);
    }
}
// error: can't prove `v == n` for `x`
```

A place holding a closure or a function holds another only once
something writes it: an assignment writes the new value, and a `&mut`
lent to a call is forgotten after it, as anything lent is. A closure's
own call can't replace it, as its body has no way to name the closure.

```rust,error
fn five() -> { v: u32 | v > 0 } { 5 }

fn zero() -> u32 { 0 }

fn set<T: Discard>(a: &mut T, b: T) {
    *a = b;
}

fn main() {
    let mut p = five;
    set(&mut p, zero);
    let x: { v: u32 | v > 0 } = p();
    let _ = x;
}
// error: can't prove `v > 0` for `x`
```

A closure a call gives back is a new value, nothing known of it, even
one the called closure's summary made. Its summary would be of values
that run made, which the next run makes anew:

```rust,error
fn main() {
    let mk = |k: u32| move || k;
    let f = mk(0);
    let g = mk(5);
    if f() == 0 {
        let x: { v: u32 | v == 0 } = g();
        let _ = x;
    }
}
// error: can't prove `v == 0` for `x`
```

## Why it's sound

> **Lemma V1 (a closure's value is of every run).** The walk where a
> closure is made is of every run of its body (S3, with step 74's fix):
> what it captured by value is as it was made, or, if the body changes
> it, a value nothing's known of but its refinement; what it captured
> by `&` can't change while it lives; what it captured by `&mut` was
> forgotten by the capture's loan. So its summary holds of each run,
> whenever the closure is called (C1). The summary is kept with the value,
> and a value never changes: wherever it's moved or copied, it's that
> closure. A place is known to hold it until the place is written:
> assigned, or lent `&mut` to a call, after which the walk knows nothing
> of it. Called through `&mut f`, `f` is the same closure after: the only
> way to write `f` is to name it, which its body can't. A summary of a
> closure the walk made of another run's values, one a call gives back, is
> never copied with it (`freshen_obj`).

> **Lemma V2 (arguments).** Walked where it's made, a closure's
> parameters are values nothing's known of but their types: closures'
> parameters can't be refined. So what the walk knows where the body
> returns holds of any values of them. For a run with arguments `a`,
> there are values for what the run made, the parameters among them,
> such that it holds with the parameters `a`: saying each integer or
> `bool` parameter is its argument says no more than that. A closure
> walked at a call, given what the call's arguments have of its
> parameters, has no such parameters (`Summary::params` is `None`): it's
> given to that call, never called straight.

> **Lemma V3 (a function's value is the function).** A function's value
> is the same value wherever it's used, so one object stands for it. Its
> parameters have no refinements, so a call through the value proves
> nothing a call of the function by name wouldn't, and what the
> function's signature says of its result and of what it's lent holds of
> every call of it, however it's reached.

> **Lemma V4 (a function given for `F`).** The closure `|xs| g(xs)`,
> written in the call, would be walked there with its parameters having
> what the call's instantiation says the callee can give them (lemma A2
> of docs/60), and none where `F` is anywhere else in the callee's
> signature, as for any closure (`closure_trees`). Its walk is one call
> of `g`. Its summary, of new variables each time it's counted on, is
> then of every call the callee makes of `g`, which C2 counts on as it
> does for a closure.

> **Lemma V5 (the only `F`).** A summary says what calling one value
> gives. Counted on for every call a callee makes of its `F` (C2 of
> docs/74), it must be the only value of `F` the callee can call: one
> parameter's type is `F`, no other parameter's type has `F` in it, and
> no bound or associated type of the callee's makes one
> (`targs::sole_source`). Two closures of one type are two values: one
> written in the call and one made by the same expression on a loop's
> turn before, given beside it in an `Option<F>`, or two variables given
> for two parameters of type `F`, or a `&List<F>`, or an iterator whose
> `Item` is `F`.

Looking for what this step could break found that step 74 broke V5: since
0.89.0, a closure written in the call was summarized even with another
of its type beside it, which a loop can make. The checker accepted this,
and the interpreter's oracle caught it:

```rust,error
fn h<R, F: Fn() -> R + Discard>(f: F, other: Option<F>) -> (R, F) {
    match other {
        Some(g) => (g(), f),
        None => {
            let r = f();
            (r, f)
        },
    }
}

fn main() with panic {
    let mut kept = None;
    for n in 0..4usize {
        let (r, back) = h(move || n, kept);
        let x: { v: usize | v == n } = r;
        let _ = x;
        kept = Some(back);
    }
}
// error: can't prove `v == n` for `x`
```

The adversarial tests are `tests/t/err_soundness_callable_*.t` (and
`err_soundness_closure_result_other_value.t`, the program above), and the
generated family `tests/generated/summaries.rs`: one program written in
eleven forms (a closure written in the call, made before it, moved, captured
by another, kept in a field, a literal, either of two returns; a function by
name, through a variable, a method), given to `call`, `twice` and
`unwrap_or_else` or called straight, each keeping or breaking the
refinement: every form gets the same verdict, and the interpreter's
oracle never fires.

## What it doesn't do

- **An iterator's `map`.** `l.iter().map(f)` keeps `f` in the adapter,
  and its items come out of `next`, a call `f` isn't given to. Its
  summary would have to ride in the adapter's type.
- **A closure made before the call, with what the call gives it.** `let
  f = |v| half(v); app(x, f)` was walked where `f` was made, its
  parameter any `u32`, so its result is any `u32`, though `app(x, |v|
  half(v))` and `app(x, half)` are below 10. Walking it again at the call
  would be sound only if every run of it were at that call.
- **Closures given by reference.** A callee taking `f: &F` gets no
  summary: only an argument whose type is `F` itself does.
- **What a call gives back.** A closure or function a call returns,
  `impl Fn` or a closure's own, is a new value nothing's known of.

## Decisions

- **D1: closures and functions, given and called.** Both kinds of value,
  wherever they go, used both where they're given for `F` and where
  they're called; not iterator adapters.
- **D2: a function given for `F` is the closure calling it.** Its
  parameters have what the call gives them, as `|xs| g(xs)` written
  there would; not only its signature, with any values.
- **D3: arguments.** A call of a closure's value says its integer and
  `bool` parameters are the arguments; others are any values.
- **D4: walked once.** A closure made before its call is walked where
  it's made, never again with what a call gives it.
- **V5 is a fix, not a choice.** A callee given two values of `F`, or a
  way to get another, gets no summary for `F`: its `R` is what step 60
  said, any value, where a call of `F` gives it.

## Exercises

1. `err_soundness_callable_inner.t` makes two closures with one call
   each of `mk`. Copy what `Walk::callables` says of an object to the new
   one `freshen_obj` makes for it. Which variable do the two closures'
   summaries then share, and why does `g() == 0` follow from `f() == 0`?
2. Why is it sound for a call of a closure's value to say its parameters
   are the arguments, but not for one walked at a call, with what the call
   gives them? Write a program a closure of the second kind would break if
   called straight. (Can it be?)
3. A closure's value called through `&mut f` keeps `f`'s value after,
   but a `&mut f` given to `set` doesn't. What does the borrow checker
   guarantee about the first that it doesn't about the second?
4. `call(five)` walks one call of `five`, through `Walk::fn_summary`,
   from what's known at the call. Why does that walk drop the errors it
   finds, and the summary with them, rather than report them?
5. Sketch what it would take for `l.iter().map(f)`'s items to have what
   `f`'s summary says. Where would the summary live, and what in `next`'s
   signature would have to say it?

## Reading

- Reynolds, *Definitional Interpreters for Higher-Order Programming
  Languages* (1972): a closure as a value, its code with what it
  captured.
- Shivers, *Control-Flow Analysis of Higher-Order Languages* (1991):
  which functions a value of function type can be, followed through the
  program, of which this is the simplest case, one per value.
- Plotkin, *Call-by-name, call-by-value and the λ-calculus* (1975):
  the η rule, by which `g` is `|x| g(x)`.
- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008): refinements of
  closures and their results, inferred.
