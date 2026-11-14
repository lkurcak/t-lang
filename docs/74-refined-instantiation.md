# Step 74: what a generic call gives back

Code: `Walk::lent_decl` and `Walk::instantiate_one` in
`src/check/refine/targs.rs`, with `makes`; `src/check/refine/summary.rs`
(`Summary`, `Made`, `Walk::summary`, `Walk::made_formula`, the
`freshen_*`s); `Walk::walk_closure`, `Walk::aggregate` and the walk of a
closure's `Return` in `src/check/refine/mir.rs`; the parameters' names in
`src/check.rs` and `src/check/closures.rs`.

t-life, the first program to run on several threads
(docs/41-threads.md), found that a generic function forgot what was
known of the values it gave back in two places where it shouldn't:

- `core::mem::swap(&mut a, &mut b)` of two `{ v: u32 | v > 0 }`s left
  each a number nothing was known of. Step 60 kept what a list behind a
  `&mut` held (docs/60-refined-type-arguments.md), but not what's
  declared of the place lent itself.
- `call(|| five())`, of a `five` giving back a `{ v: u32 | v > 0 }`, gave
  back any number: nothing went through a closure
  (docs/55-free-theorems.md). `Mutex::lock`, a thread's `join` and a
  scoped pool give back what a closure did, and all lost it.

Now both keep it:

```rust
use core::mem::swap;
use core::sync::Mutex;

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn five() -> { v: u32 | v > 0 } { 5 }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: { v: u32 | v > 0 } = 1;
    let mut b: { v: u32 | v > 0 } = call(|| five());
    swap(&mut a, &mut b);
    let m = Mutex::new(12u32);
    let c: { v: u32 | v <= 10 } = m.lock(|n| {
        if *n > 10 {
            return 10;
        }
        *n
    });
    let h = os.threads.spawn(|| five());
    let d: { v: u32 | v > 0 } = h.join();
    out.line(&f"{a} {b} {c} {d}") // expect: 5 1 10 5
}
```

## The place lent, itself

A call's instantiation of a type parameter `X` is, at each position of
the type it's given, what the arguments have there (lemma A3 of
docs/60): what's known of the values, and, behind a `&mut`, what's
declared of the place lent, which is what the callee may find there. For
`swap`, `X` is `{ v: u32 | v > 0 }`'s place's type: but what was declared
of the place counted only the refinements of its type arguments, not the
place's own. So the instantiation said nothing of the values, and the
place's refinement, proved again after the call, couldn't be.

What's declared of a place lent straight to a call (`Walk::lent_decl`)
now has the place's own refinement too, at the reference's position of
what it points to: a refined variable's, or a field's, of the struct's
fields now. The instantiation of `swap`'s `T` is then what both places
declare, which `swap` writes back to both:

```rust,error
use core::mem::swap;

fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    let mut b: { v: u32 | v > 1 } = 2;
    swap(&mut a, &mut b);
}
// error: can't prove `v > 1` of what `swap` may write behind argument `b`
// error: can't prove `v > 1` for `b` after the call to `swap`
```

The first error is the instantiation's: `a`'s 1 may end up in `b`. The
second is the refinement of `b` proved again after the call, as before.

A field's refinement may name the struct's other fields
(docs/08.1-field-refinements.md): what the call is lent of it is the
refinement with those fields' values as they are at the call, a fixed
predicate. If one of those fields is lent to the call too, and changed,
the struct's refinements, proved again of the fields after the call,
find it:

```rust,error
use alloc::list::List;

fn put<T, U: Discard>(_a: &mut T, b: &mut U, x: U) {
    *b = x;
}

#[derive(Discard)]
struct Band { rows: usize, now: { l: List<u8> | l.len() == rows } }

fn main() with alloc {
    let mut band = Band { rows: 2, now: List::filled(2, 0) };
    put(&mut band.now, &mut band.rows, 5);
}
// error: can't prove `l.len() == rows` for field `now` of `Band`
```

## What a closure gives back

A call's type parameter `R` that's in the result of a closure it's
given, `F: FnOnce() -> R`, was what step 60 called *made*: a value of it
can come from calling `F`, which the call couldn't follow. But a closure
written in the call is walked there, from what's known there (lemma S3 of
docs/69-loans.md), as every run of its body: where it returns, the walk
knows what it gives.

That's its *summary* (`summary::Summary`): for each place its body
returns, the value it gives there and what's known there. It's one more
case of `R`'s instantiation (`Cand::Made`), with those of the call's
other arguments of type `R`. `call(|| five())` gives back a value with
what's known where the closure returns: `five`'s result, which is
greater than 0.

A summary can't be said as a predicate of the call's own: it names the
values the run made, a local's, a parameter's, the result of a call in
it. Another run makes others. So they're said to *exist*: each time a
summary is counted on, of a value the call gave, they're new variables
of the logic. Two values a closure gave, on two runs, aren't the same:

```rust,error
fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn main() {
    let mut k: u32 = 0;
    let (a, b) = twice(|| {
        k = k / 2 + 1;
        k
    });
    let c: { v: u32 | v == a } = b;
    let _ = c;
}
// error: can't prove `v == a` for `c`
```

What it names from before its walk is the same in every run, as values
never change: what it captured by value, a measure of what it captured
by `&`. So `call(move || k + 1)`, of a `k` below 100, is below 101.

A closure given for `F` may be called with any value the call has of
its parameters' types: what the closure's parameters are assumed to be is
what the call's instantiation says (step 60's D6), so what it gives back
is what it does of those. Where a bound on the parameter's type makes
values, the closure's parameter is any value, and so is what it gives:

```rust,error
use core::num::{Int, greatest};

fn app<T: Int, R, F: FnOnce(T) -> R + Discard>(_x: T, f: F) -> R { f(greatest::<T>()) }

fn main() {
    let x: { v: u32 | v < 10 } = 4;
    let c: { v: u32 | v < 10 } = app(x, |z| z);
    let _ = c;
}
// error: can't prove `v < 10` for `c`
```

## Captures, as they were made

Looking for what a summary could count on found two holes in the walk
of a closure's body, both since 0.84.0, which made each run of its body
see values it didn't:

- A closure that changes what it captured by value was walked as if each
  run saw the value it was made with. `move || { let r = a[n]; n = n + 2;
  r }`, run three times, indexed `a[4]` of a three-element array, and was
  accepted.
- A closure given straight to a call is walked at the call, after the
  arguments after it: it read what it captured by value as it was then,
  not as it was made. `call2(move || a[n], { n = 1; })`, of an `n` of 7,
  was accepted, and indexed `a[7]`.

Now the walk takes what a closure captures where it's made, and starts
each walk of its body from that; but what its body changes, which each
run reads as the runs before left it, is a value nothing's known of but
what's declared of it:

```rust,error
fn thrice<F: FnMut() -> u32 + Discard>(mut f: F) -> u32 {
    let _ = f();
    let _ = f();
    f()
}

fn main() {
    let a: [u32; 3] = [1, 2, 3];
    let mut n: usize = 0;
    let _ = thrice(move || {
        let r = a[n];
        n = n + 2;
        r
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

A refined one keeps its refinement, which each write must keep: with `n:
{ v: usize | v < 3 }`, `a[n]` is fine and `n = 2` is checked.

## `_`

A parameter named `_` binds nothing, so there can be as many as a
function or a closure likes: `|_, _| 4` and `fn k(_: u8, _: u8)` were
refused as declaring `_` twice.

## Why it's sound

- **A3 at the place itself.** A place's own refinement holds of its value
  whenever it can be read (lemma A6 of docs/60), so of what the callee
  finds behind the `&mut`: it's a case of the instantiation. By
  parametricity (A2) the callee writes back only values of the
  instantiation; the place's refinement must follow from it (`Behind`),
  and is proved again after the call (`Walk::post_now`), the struct's of
  its fields' values after (`prove_fields`). A local's refinement names
  only what can't change (lemma A1); a field's, the struct's fields, whose
  values at the call make it a fixed predicate.
- **S3, a closure's body sees what it will.** A closure's body is walked
  from what's known where it's made, or at the call it's given to, with
  what it captured by value as it was made. What its body changes of
  that, each run reads as a new value, with only what's declared of it;
  what it captured by `&` can't change while it lives; what it captured
  by `&mut` was forgotten by the capture's loan. So what the walk knows
  at each point of the body holds of every run, at that point.
- **C1, a summary is of every run.** By S3, where the body returns, what
  the walk knows is true of each run, of the values that run made; and of
  those from before, which are the same values in every run, as values
  never change. So each value a run gives has, of some values for those
  the run made, what's known there. Saying so of new variables each time
  says nothing of two runs' values being the same, which they needn't be.
  What the walk made that's a function of a value from before (a measure,
  an enum's part, a constant of the function) is the same in every run,
  and kept.
- **C2, what a generic call gives.** A function generic in `R`, whose
  bounds on `R` are markers and consumers, gives back and writes only
  values of `R` it was given or got from calling a closure it was given
  (Reynolds' abstraction theorem, with what the closure gives related as
  what it's given is: lemma A2 of docs/60, with the closure as one more
  argument). Each such value is one of the instantiation's cases: an
  argument's, or one of a closure's summary's.
- **The interpreter checks.** A refined `let` of what a call gave is
  checked when it runs (`t run --interp`), as is a refined variable each
  time it's used after a call it was lent to.

The adversarial tests are `tests/t/err_soundness_lend_top_*.t`,
`err_soundness_closure_result_*.t` and `err_soundness_capture_*.t`.

## What it doesn't do

- **Functions given for `F`.** `call(five)` gives back any number: a
  function as a value is a `fn` pointer, and which function it is isn't
  followed. Its signature would be its summary.
- **Closures made before the call.** `let f = || five(); call(f)` gives
  back any number: the walk follows only a closure written in the call.
  Keeping a summary with the closure's value would follow it further.
- **An iterator's `map`.** `l.iter().map(|x| ..)` keeps its closure in
  the iterator, and its items come out of `next`, a call it's no longer
  given to.
- **One message for one failure.** A lent place whose refinement the
  instantiation can't keep is reported twice, at the call and when it's
  proved again after.

## Decisions

- **D1: a closure's result is what its body knows.** Not only what a
  refinement written on its result type says, which the closures
  t-life writes don't have.
- **D2: only closures written in the call.** Not functions given as
  values, nor closures made before it, for now.
- **D3: locals and fields.** A field whose refinement names other fields
  lends it at their values now, not only a field whose refinement is about
  it alone (docs/67-refinement-aliases.md).

## Exercises

1. Make `made_formula` use a summary's values as they are, without new
   variables for those the run made. Which test fails, and what does the
   program it checks do?
2. Keep summaries by the closure's type, so that `let f = || ..; call(f)`
   has one. Write a program with a closure made in a loop, and called
   after it, that would then be wrongly accepted.
3. A field lent to `swap` is lent at its struct's other fields' values
   at the call. What checks that one of those, lent to the same call and
   changed, doesn't leave the field's refinement false?
4. Why does the walk forget the captures a closure's body changes, and
   not every capture of a closure that's `FnMut`?
5. What would it take for `l.iter().map(|x| ..)`'s items to have what
   the closure's summary says?

## Reading

- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008): refinements of
  closures and their results, inferred.
- Knowles and Flanagan, *Compositional Reasoning and Decidable Checking
  for Dependent Contract Types* (2009): existential types for what a
  term's result is known to be, of the values it made.
- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983), and
  Wadler, *Theorems for free!* (1989): what a function's type says of
  what it does with the functions it's given.
- Dijkstra, *A Discipline of Programming* (1976): the strongest
  postcondition, which is what a summary is, of the closure's body.
- The Rust Reference, *Closure expressions*: what a closure captures, and
  how.
