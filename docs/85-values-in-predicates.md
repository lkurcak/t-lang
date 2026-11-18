# Step 85: values in predicates

Code: `Checker::pred_operand`, `pred_path` and the `*` in `pred_place`
(`src/check/refine.rs`); `Walk::value_eq`, `Walk::definitions`,
`Walk::at_return` and `Walk::left`, called from `Walk::post_now`; in
`src/interp.rs`, `Interp::model` and `each_value_side`; the signatures of
`swap` and `replace` in `core/mem.t`, and of `Option::take` in
`core/option.t`.

A predicate could name an integer, a `bool`, a field, or a measure of a
value, but never a value whole. So a function couldn't say which value it
leaves behind a `&mut`, or which one it gives back. `mem::replace` gives
back what was there, but its signature only said "a `T`", and everything
known of the value was lost on the way: until this step, `old[0]` here was
refused, as nothing said `old` had a value in it.

```rust
use alloc::list::List;
use core::mem::replace;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut buckets: List<u8> = List::new();
    buckets.push(7);
    let old = replace(&mut buckets, List::new());
    out.line(&old[0])?;        // expect: 7
    out.line(&buckets.len())?; // expect: 0
    Ok(())
}
```

And `swap`, whose body is the machine's, was the one function the
refinement checker knew by name (docs/84-counting.md): it wrote each
place's value to the other. Now both say what they do, in their
signatures, and so can any function.

## `*r`

A predicate can name what a reference or a box points to, as it names a
field: `*r`. Through a `&mut`, it's what's there at return in a
postcondition, and `old(*r)` is what was there at the call, as for fields
(docs/30-postconditions.md):

```rust
/// One more, of what `r` points to.
fn bump(r: { r: &mut u8 | *r < 255 }) ensures *r == old(*r) + 1 {
    *r = *r + 1;
}

fn below(n: { n: u8 | n < 8 }) -> u8 { n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut n: u8 = 5;
    bump(&mut n);
    bump(&mut n);
    out.line(&below(n))?; // expect: 7
    Ok(())
}
```

What a slice or a `str` is has no size of its own: the reference says how
long it is, so a predicate names it by the reference's measures, `s.len()`,
not as `*s`.

## Whole values, compared

Two values of any one type can be compared with `==` or `!=` in a
predicate, as a whole: `*a == old(*b)`. It's not the type's `Eq`, which is
code a program writes, but whether they're **the same value**, as the logic
sees values:

- integers, `bool`s and characters, as themselves;
- a struct or a tuple, by its fields; an enum, by which variant it is and
  that variant's fields; an array, by its elements;
- a reference, by where it points (and, of a slice's, how long it is), not
  by what's there;
- a box, by what it owns, which is part of its value;
- what's in an `Unsafe`, as a `Cell`'s, `Mutex`'s or atomic's value is, not
  at all: it can change behind a shared reference, so no predicate can see
  it.

So the same value is one nothing a predicate can say tells apart from the
other. That's what a function generic in `T` can promise of a `T`, which it
can't look into: `swap`'s signature is now

```rust,module
#[intrinsic]
export fn swap<T>(a: &mut T, b: &mut T) ensures *a == old(*b), *b == old(*a);
```

and `replace`'s

```rust,module
export fn replace<T>(dest: &mut T, src: T) -> { r: T | r == old(*dest) } ensures *dest == src {
    let mut s = src;
    swap(dest, &mut s);
    s
}
```

A whole value alone, with no `==`, is still an error: it isn't an integer
or a `bool`, which is all the logic's arithmetic and connectives take.

## What a caller learns

A part of a postcondition that's an `==` of whole values, one side of which
is *at return*, defines it: the result, or what a call-scoped `&mut`
parameter points to, or a field of that. After the call, the place lent is
that value, with everything known of it, and so is the result:

```rust
use alloc::list::List;
use core::mem::swap;

#[derive(Copy, Discard)]
struct Pair { lo: u8, hi: u8 }

/// Each of three gets the next one's value.
fn rotate<T>(a: &mut T, b: &mut T, c: &mut T) ensures *a == old(*b), *b == old(*c), *c == old(*a) {
    swap(a, b);
    swap(b, c);
}

/// Copies a pair, field by field: the same value.
fn copy_into(dst: &mut Pair, src: &Pair) ensures *dst == *src {
    *dst = Pair { lo: src.lo, hi: src.hi };
}

fn below(n: { n: u8 | n < 8 }) -> u8 { n }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut x: List<u8> = List::new();
    let mut y: List<u8> = List::new();
    y.push(5);
    let mut z: List<u8> = List::new();
    z.push(6);
    z.push(7);
    rotate(&mut x, &mut y, &mut z);
    out.line(&x[0])?; // expect: 5
    out.line(&y[1])?; // expect: 7
    let mut d = Pair { lo: 0, hi: 0 };
    copy_into(&mut d, &Pair { lo: 1, hi: 2 });
    out.line(&below(d.hi))?; // expect: 2
    Ok(())
}
```

The function proves its word: `rotate`'s by the two `swap`s, each of which
leaves exactly the value it says; `copy_into`'s by the fields, which are
the pair. Breaking it is refused:

```rust,error
use alloc::list::List;
use core::mem::swap;

fn change(a: &mut List<u8>, b: &mut List<u8>) with alloc ensures *a == old(*b) {
    swap(a, b);
    a.push(1);
}
// error: can't prove `*a == old(*b)` for what `change` ensures
```

Anywhere else, in a precondition, a refinement or an invariant, or inside
an `||`, an `==` of whole values is a formula, which is true of one value,
and of structs and enums as their parts are: so `{ r: Pair | r == a || r ==
b }` says what `r.lo` can be. Of two values nothing relates, it's a fact of
its own, which proves nothing and refutes nothing.

`Option::take` says it gives back what was there and leaves `None`:

```rust
fn inner(o: { o: Option<u8> | o.is_some() }) -> u8 {
    match o {
        Some(x) => x,
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut o: Option<u8> = Some(3);
    let t = o.take();
    out.line(&inner(t))?;    // expect: 3
    out.line(&o.is_none())?; // expect: true
    Ok(())
}
```

## Why it's sound

- **V1 (congruence).** Every predicate is a function of what the list
  above compares: a measure's body reads fields and other measures, a
  slice's length is the reference's, and nothing reads what's in an
  `Unsafe`. So two values that are the same have every refinement, and
  every measure, alike.
- **V2 (objects are values).** The walk gives each value an object, and a
  place a new one wherever it may change: when it's written, or lent
  `&mut`, and so is what it's in, up to a reference's `*` (a reference is
  the same one whatever is written through it) but through a box's (what
  a box owns is part of it). A shared borrow keeps a place's object, as
  through it only what's in an `Unsafe` can change, which isn't part of
  the value. So one object is one value.
- **V3 (proving).** `==` is proved true of one object, of structs whose
  fields are, of enums with the same variant and fields, and of integers
  and `bool`s as the logic says; each of these is the same value. Of
  anything else it's a `bool` of its own, which can't be proved.
- **V4 (knowing).** What's known of two values that are the same is what
  V3 says, and that each measure of one known so far is the other's: all
  true of the same value. The `bool` of its own says nothing of anything.
- **V5 (definitions).** A caller writes a place it lent with the value its
  callee says is there, which the callee proved (or, of `swap`, which the
  machine does, and the interpreter checks at every call). What's lent to
  a call-scoped parameter is what the callee left there, nothing having
  borrowed it after (docs/30), and the walk had forgotten it, so nothing
  else said what it was. The value was found at the call, and is the same
  value now; what a shared reference in it pointed to can't have changed
  while it's borrowed, and what a box owns is part of the value, but what
  a `&mut` in it points to may have been written through it since, which
  is forgotten (`Walk::left`; `err_soundness_values_mut_snapshot.t`,
  which the interpreter caught as this step was written).
- **V6 (swap).** V5 of `swap`'s signature is what the walk did of `swap`
  since step 84 (M1 of docs/84): `*a` gets `*b`'s value, and the other
  way round. It's as trusted as it was, being an intrinsic's, and the
  interpreter now checks it, comparing the values whole.

## What it doesn't do

- A whole value that isn't defined at return, in a precondition or an
  invariant, is known only by its parts and its measures known so far: no
  equation between two values is kept (congruence closure, which would,
  is more machinery than any program needs yet).
- An array's elements aren't followed (docs/22), so two arrays are the
  same value only if they're one.
- A list's elements aren't in its value: `List::replace(i, v)` still says
  nothing of what it gives back, as `l[i]` isn't in the logic.
- `Cell::replace` takes `&self`: what's in a cell isn't in the logic.

## Decisions

- **D1: the same value, as the logic sees values.** Not `Eq`, which is a
  program's code, and not "only types that derive `Eq`", which would leave
  out `swap`'s `T`. A cell is the same value whatever's put in it, as no
  predicate can see that.
- **D2: definitions from postconditions.** Exact where a callee says it,
  as `swap` and `replace` do; elsewhere, a formula of the parts. A
  union-find of equal objects, kept along each path and intersected
  where paths meet, would know more, and no program needs it yet.
- **D3: `for x in x: e` warns** (`shadowed_iterators`): in the body `x`
  is the value, in the invariant the iterator, which reads as one thing.

## Exercises

1. Write `exchange(a: &mut (u8, u8))` that swaps the two halves of a pair,
   with an `ensures` the caller can use. Can you say it without `==` of
   whole values?
2. `fn first<T>(a: T, b: T) -> { r: T | r == a }` can't be written for a
   `T` that isn't `Discard`. Why? Which values could it give back?
3. Take `ensures self.is_none()` off `Option::take`. What does
   `values_replace.t` stop proving?
4. A function says `ensures *a == old(*b)` of two `&mut Box<u8>`, and
   swaps them, then writes `**a = 5`. Why is that refused, and what would
   go wrong if it weren't? What if they were `&mut &mut u8`?
5. `copy_into` says `*dst == *src`, of what `src` points to at return.
   Why is that `old(*src)` too? Would it be if `src` were a `&mut Pair`?

## Reading

- Denis, Jourdan and Marché, *Creusot: a Foundry for the Deductive
  Verification of Rust Programs* (2022): `==` in specifications as
  equality of logical models, not `PartialEq`.
- Astrauskas, Müller, Poli and Summers, *Leveraging Rust Types for Modular
  Specification and Verification* (Prusti, 2019), and Prusti's snapshot
  equality `===`, which compares values whole.
- Lattuada et al., *Verus: Verifying Rust Programs using Linear Ghost
  Types* (2023): spec `==` and extensional equality.
- Ackermann, *Solvable Cases of the Decision Problem* (1954), and Nelson
  and Oppen, *Fast Decision Procedures Based on Congruence Closure*
  (1980): equality with functions, what V4 does for the measures known,
  and congruence closure what D2 leaves out.
- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983): why
  a function generic in `T` can only promise which of its `T`s it gives.
