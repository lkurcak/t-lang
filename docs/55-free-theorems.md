# Step 55: free theorems

Code: `free_params` in `src/check/refine/free.rs`; `Checker::free_of`
and `Walk::free_theorems`, `Walk::free_part` in `src/check/refine.rs`;
`free_theorems` in `src/interp.rs`; `checked_*` in `core/num.t`,
`Option::ok_or` in `core/option.t`.

Step 53 let a function say what's in the `Option` it gives back. What
`unwrap` gave of it was a value nothing was known of, and so was what `?`
gave:

```rust,error
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn same(i: usize) -> usize { i }

fn at(a: [u8; 4], n: usize) -> u8 with panic { a[small(n).map(same).unwrap()] }
// error: can't prove `i < self.len()` for argument `i` of `index`
```

Worse, a function whose own result was a refined `Option` couldn't use
`?` at all: what it returns early, `Try::from_residual(r)`, was a value
nothing was known of, so the checker couldn't show it has the result's
refinement, though it's always a `None`. Without the `map` above, and in
such a function, both are proved now:

```rust
fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

fn at(a: [u8; 4], n: usize) -> u8 with panic { a[small(n).unwrap()] }

fn double(n: usize) -> Option<{ v: usize | v < 7 }> {
    let i = small(n)?;
    Some(i + i)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a: [u8; 4] = [5, 6, 7, 8];
    out.line(&at(a, 2))?;               // expect: 7
    out.line(&double(3).unwrap_or(0))?; // expect: 6
    out.line(&double(9).unwrap_or(0))?; // expect: 0
    Ok(())
}
```

## What a generic function can't do

`unwrap` is `fn unwrap(self: Option<T>) -> T`, for every `T`. It can't
look at a `T`, add to one, or make one: all it knows of `T` is that it's
a type. So the `T` it gives back must be one it was given, and the only
one is in the `Some`. Wadler called this a *free theorem*: what a
function's type alone says it does, whatever its body.

A type parameter `T` of a function is *free* when:

- each trait bound on it is a marker, `Copy`, `Discard`, `Send` or
  `Sync`, which can copy a value, drop one or send one to another
  thread, but never make one;
- it's in no other bound, as `I: Iterator<Item = T>` is, whose `next`
  makes `T`s, or `F: FnOnce() -> T`;
- and everywhere it is in the parameters' and result's types, it's on a
  *way*: `T` itself, or a field of an enum's variant, through enums'
  type arguments, as in `Option<T>`, `Result<T, E>` or
  `ControlFlow<Result<!, E>, T>`. Not behind a `&` or `&mut` (what's
  behind one can change, `mem::replace` gives back what was there), nor in
  a struct (whose fields may be private: what callers prove would then
  depend on them), an array, a box or a closure.

For a method of an impl of a trait, what the impl says the trait's
associated types are (`Option<T>`'s `Output` is `T`) isn't a bound: a
value of one is made only by the impl's methods, which can make a `T` no
more than this one can.

Then at a call, each value of `T` in the result is one of the values of
`T` in the arguments. The checker knows what an enum's value is made of
(step 53), so it says so of the parts:

- `o.unwrap()` is what's in `o`, and `o` was a `Some`, once it's
  returned; `r.expect(..)` the same of an `Ok`.
- `o.unwrap_or(d)` is what's in `o`, if it's a `Some`, or `d`.
- `r.ok()`'s `Some` holds what `r`'s `Ok` did, and is only a `Some` if
  `r` was an `Ok`; `o.ok_or(e)` the same the other way, its `Err` is `e`;
  `r.err()`, `r.into_ok()`.
- `pick(c, a, b)`, with `fn pick<T: Copy>(c: bool, a: T, b: T) -> T`, is
  `a` or `b`.
- A part of the result with no part of the arguments it could be isn't
  there: `Try::from_residual(r)`, of an `Option<!>`, is a `None`.

`?` is `Try::branch` and `Try::from_residual` (docs/15-try.md), core's
impls for `Option` and `Result`, which the checker finds at a call whose
type it knows (lemma R1 of docs/53). So what `?` gives keeps the
refinement of what's in the `Some` or `Ok`, a function whose result is
refined can use `?`, and an error's refinement goes up through it:

```rust
fn sorted(n: usize) -> Result<{ v: usize | v < 4 }, { e: usize | e >= 4 }> {
    if n < 4 { Ok(n) } else { Err(n) }
}

fn up(n: usize) -> Result<usize, { e: usize | e >= 4 }> {
    let i = sorted(n)?;
    Ok(i)
}

fn past(n: usize) -> usize {
    match up(n) {
        Ok(_) => 0,
        Err(e) => e - 4,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&past(6))?; // expect: 2
    out.line(&past(1))?; // expect: 0
    Ok(())
}
```

Where there's one value the part could be, it *is* that value, the same
object (docs/53): so what's known of it besides its refinement comes
along, as a slice's length does through `unwrap`. Where there are
several, it's equal to one of them, each under the condition that it's
there.

The arithmetic that can't overflow now says what it gives: `checked_add`
is `Option<{ v: T | v == a + b }>`, and `checked_sub`, `checked_mul`,
`checked_neg` and `checked_abs` the same. So their `?` keeps it:

```rust
fn product(t: [u8; 10], i: { v: usize | v < 4 }, j: { v: usize | v < 4 }) -> Option<u8> {
    let k = checked_mul(i, j)?;
    Some(t[k])
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    out.line(&product(t, 3, 3).unwrap_or(0))?; // expect: 9
    Ok(())
}
```

A bound that can make a value makes no theorem. A `T: Int` can be a
literal:

```rust,error
fn nine<T: Int>(x: T) -> T {
    let _ = x;
    9
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[nine(i)] }
// error: can't prove `i < self.len()` for argument `i` of `index`
```

## Why it's sound

- **Lemma P1 (parametricity).** Reynolds' abstraction theorem: a function
  generic in `T` treats values of `T` alike, so it relates its arguments
  to its result by any relation it's given that its bounds respect. Take
  the relation that's the identity on the values of `T` in the
  arguments: markers respect it (they make nothing), so each value of `T`
  in the result is one of those. It needs that nothing in T can make a
  `T` from nothing or tell `T`s apart by their type. There's no `match`
  on a type, no specialisation, and no transmute; an impl is chosen by
  coherence, the same for every value of a type; and each intrinsic
  generic in a `T` with only markers moves, copies, drops or stores the
  values it's given (`swap`, `RawBuf`'s `write` and `read`, whose slots
  hold only what was written, `Unsafe`, an arena, a thread's result). The
  one that reads a `T` from an address, `Volatile::at`, needs `T: Atom`,
  which isn't a marker. Unsafe code reading a slot nothing was written to
  breaks its own obligation already.
- **Lemma P2 (the facts).** An enum's value is its variant and fields
  (lemma E1 of docs/53), so a way's part is there exactly when each enum
  on the way is that variant, and P1 is exactly: if a result's part is
  there, it equals one of the arguments' parts that are there. One
  candidate: the part is that value, and those conditions hold; a part
  that isn't there is no value, so it may be any, that one included.
  None: the part isn't there, and a whole result of `T` means the call
  doesn't return.
- **Lemma P3 (`?`).** `branch` and `from_residual` are core's impl
  methods, found by R1; `T` is free in both, so `?`'s value is what's in
  the `Some` or `Ok`, `from_residual`'s `T` has no candidate (an
  `Option<!>` holds none), so it's a `None` or an `Err`, and the `Err`'s
  error is the one `branch` broke with.
- **The interpreter checks.** At each call of a function with a free
  type parameter, `t run --interp` checks that each part of the result
  that's there is one of the arguments' parts that are there, by value
  (references by where they point). Every test program runs it.

## What it doesn't do

- Nothing goes through a closure: `map`, `and_then` and `unwrap_or_else`
  give what the closure gives, which a free theorem can't say anything of
  (its `T` is in the closure's bound). Since step 74, a closure written in
  the call says what it gives by its summary, as one case of the
  instantiation (docs/74-refined-instantiation.md).
- Nor behind a reference: `copied`, `as_ref` and `take` say nothing; nor
  in a struct: `Cell::into_inner`, a `List`'s `pop`.
- `?` on an `Option<T>` in generic code: which impl it is can't be known
  where the type has a type parameter (R1), so it says nothing there.
- Where several values could be the one, what's known of them besides
  their integers and enums' parts (a measure) wasn't carried over; since
  step 56 it is (lemma S2 of docs/56-field-paths-and-slices.md).
- A dead `match` arm is still only found by a goal that shares a variable
  with what makes it dead: `None => a[9]`, after `o.unwrap()`, is still
  an error.
- `split_at`, `str::get` and `find` didn't say how long what they give
  is: that needed a refinement that can name a struct's fields, which
  step 56 added, and with it the lengths of the first two.

## Decisions

- D1, what's free: type parameters bounded only by markers, in no other
  bound, on ways through enums. Rather than also through structs' fields
  (callers' proofs would depend on a library's private fields), or bounds
  like `Eq` whose methods can't make a `T` (each trait would need to be
  checked for that, and `Ord`'s `max` and `min` can). Step 60 checks each
  trait for that, and moved `max` and `min` out of `Ord`: consumer bounds
  count as markers now (docs/60-refined-type-arguments.md).
- D2, a part with no candidate isn't there, rather than nothing being
  known of it: that's what lets a function with a refined result use `?`.
- D3, `checked_*` say what they give, and `Option::ok_or` is new;
  `split_at`'s halves' lengths wait for refinements that can name
  fields.

## Exercises

1. What's the free theorem of `fn f<T: Copy>(a: Option<T>, b: Option<T>)
   -> Option<T>`? Write two different bodies; does the theorem tell them
   apart?
2. Why isn't `fn get<T: Copy>(r: &T) -> T` given a theorem, though it
   can only give back `*r`? Write a program that would be wrongly
   accepted if what's behind a `&mut T` parameter counted.
3. `fn swap_back<T: Discard>(x: T, y: T) -> Result<T, T>` gives `Ok(y)` or
   `Err(x)`. What can the checker say of its result? What can't it?
4. Suppose T had `fn size_of<T>() -> usize`. Does it break lemma P1?
   What about `fn zeroed<T>() -> T`?

## Reading

- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983): the
  abstraction theorem.
- Wadler, *Theorems for free!* (1989): what a polymorphic function's type
  says of every function of that type.
- Vazou, Rondon and Jhala, *Abstract Refinement Types* (2013): refinements
  that flow through polymorphic functions, as parameters of their types.
- Voigtländer, *Free Theorems Involving Type Constructor Classes* (2009),
  and Johann and Voigtländer, *Free Theorems in the Presence of seq*
  (2004): which features of a language break them.
