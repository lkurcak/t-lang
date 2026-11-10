# Step 63: what's declared behind every reference

Code: `Walk::borrow`, `Walk::lvalue`, `Walk::deref_lvalue` and the
assignment of a field in `src/check/refine.rs`, and
`RefinedFields::constrained_unplaced`; `Walk::field_decl`,
`Walk::place_read`, `Walk::instantiate_one`, `Walk::subst_tree_unknown`,
`Walk::closures_given` and `parametric_impl_depth` in
`src/check/refine/targs.rs`; `Walk::closure_decls`.

Retrospective 6, after 0.78.0, looked for holes before looking for
anything else, and found programs that wrote 99 where every value was
declared below 5. Each was a way of getting to a value that the checker
had no case for. It then took nothing to be declared there, so it checked
nothing that was written through it:

- a reference to a list's or an array's element, `&mut l[0]`, kept in a
  variable, passed on, or given to a function that takes a `&mut usize`;
- a field of an element, `lp[0].1 = 99`, of a list of `(u8, { t: usize |
  t < 5 })`;
- a tuple's element bound by a pattern, where the tuple's type refines it;
- what an iterator adapter gives, `l.iter_mut().skip(1)`, `rev`,
  `enumerate`, `zip`, `map`: a value of `I::Item`, a projection the
  checker didn't follow;
- an iterator given to a generic function whose bound pins its items,
  `I: Iterator<Item = &'a mut usize>`;
- a closure, or a function, given to `fold`, `any` or a function of one's
  own, that's called with a `&mut` to each element;
- a temporary's refined field lent `&mut` to a call.

All of them are since 0.76.0 (step 60), but the last, which is older.
This step makes the checker say, of every way a reference is made, what's
declared behind it. Where it can't say, it refuses the program rather
than assume nothing.

## What a reference says

A reference says what's declared of the value it points to: what every
write through it must keep. That was so of a reference to a variable or
a field of one, whose place the checker knows. Now it's so of every
place: what a reference a call gave points to (`l[0]` is
`*l.index_mut(0)`), a field of that, and a reference to what a reference
points to, which is that reference again, saying what it says. A value
that isn't a place, a temporary, is an anonymous `let`: it declares what
a `let` of it would.

```rust,error
use alloc::list::List;

fn reset(l: &mut List<{ t: usize | t < 5 }>) {
    if l.len() > 0 {
        let r = &mut l[0];
        *r = 99;
    }
}
// error: can't prove `t < 5` for what's written through `*r`
```

So it doesn't matter what the reference is kept as, or given to: a `let
q: &mut usize = &mut a[0]` points to what's declared below 5, as its
object says, and so does what it's passed on as. Given to a function
that takes a `&mut usize`, it's given to code that may write anything
there:

```rust,error
fn bad(r: &mut usize) { *r = 99 }

fn main() {
    let mut a: [{ t: usize | t < 5 }; 2] = [0, 1];
    bad(&mut a[0]);
}
// error: can't prove `t < 5` of what `bad` may write behind argument `r`
```

What keeps it is fine, however it gets there:

```rust
use alloc::list::List;

fn set<T: Discard>(r: &mut T, v: T) { *r = v }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let mut pairs: List<(u8, { v: usize | v < 5 })> = List::new();
    pairs.push((0, 1));
    if l.len() > 0 && pairs.len() > 0 {
        let r = &mut l[0];
        *r = 3;
        set(&mut l[0], 4);
        pairs[0].1 = 2;
        out.line(&f"{l[0]} {pairs[0].1}")?; // expect: 4 2
    }
    Ok(())
}
```

A field lent straight to a call-scoped `&mut` is proved again after the
call (step 30): a field of a temporary has no place to be proved again
at, and its destructor may count on it, so it can't be lent so, unless
all that's said of it is carried in its type arguments, which what's
written through the reference keeps.

## Given whole

`&mut T` is invariant in `T`: what's written through it must be exactly
what's declared there. So is an `IterMut<'_, T>`, which hands out `&mut
T`s, and a `Cell<T>`. A type parameter given such a value whole, `I` given
an `l.iter_mut()`, was taken to be what's known of it, though the callee
may write `T`s through what it gives. Now what's declared there is what
the callee may write there, as for a `&mut`. Two iterators of different
lists can't be one `I`; nor can an iterator be given to a function whose
bound says its items are something the function may make:

```rust,error
use alloc::list::List;

fn all_to<'a, I: Iterator<Item = &'a mut usize> + Discard>(it: I) {
    for r in it {
        *r = 99;
    }
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    all_to(l.iter_mut());
}
// error: can't prove `t < 5` of what `all_to` may write behind argument `it`
```

## Through associated types

What `next` of a `Skip<I>` gives is an `Option<I::Item>`: a projection,
whose values the checker took to be nothing it knew. Now a projection
`<X as Tr>::A` in what a call gives is what the impl of `Tr` for `X`
says `A` is, of what's at its type parameters' positions in `X`: `Iter<'_,
T>`'s `Item` is `&T`, so `l.iter().skip(1)`'s values are `&`s to what the
list's type says. An impl whose own bounds could make values of what its
parameter is (`where I::Item: Default`) says nothing so, and an impl's
bound that isn't a consumer is as good as one, if the impl of it for
what the parameter is there is parametric too: `Rev<I>`'s `I:
DoubleEndedIterator`, of an `Iter<'_, T>`.

```rust
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
    let mut l: List<{ t: usize | t < 8 }> = List::new();
    l.push(7);
    l.push(0);
    for (k, i) in l.iter().rev().enumerate() {
        out.line(&f"{k} {a[*i]}")?;
    }
    // expect: 0 10
    // expect: 1 17
    Ok(())
}
```

And what's written through what `iter_mut`'s adapters give is checked:

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    for x in l.iter_mut().rev() {
        *x = 99;
    }
}
// error: can't prove `t < 5` for what's written through `*x`
```

Where a projection's values can't be said (an impl's parameter that isn't
in the type the impl is for, which a bound decides, such as `Map`'s `B`),
nothing is known of them: and if a reference there could be written
through, the call is refused.

## Closures given

`fold`'s closure is called with each value the iterator gives: a closure
written in the call is checked with what's declared of them, as
step 60 said, so a write through a `&mut` it's given is checked. But a
closure made before the call, or a function, says only what its
parameters' types do, and a parameter of a closure, or of a function
that's used as a value, can't be refined: it may write anything. So
neither can be given where the callee gives it a `&mut` to what's
refined, nor can a closure the callee keeps, such as `map`'s, which could
be called later with something else:

```rust,error
use alloc::list::List;

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    let f = |acc: u8, x: &mut usize| {
        *x = 99;
        acc
    };
    let _ = l.iter_mut().fold(0u8, f);
}
// error: can't prove `t < 5` of what this may write
```

Each closure is its own type (step 6c's H5), so what a closure was walked
with is known wherever its type is: every call that gives one, for a type
parameter it calls with such a `&mut`, checks it, `Map`'s `next` as well
as `map`.

## Why it's sound

- **Lemma W1 (every reference says what's declared).** By cases on how a
  reference is made, each saying it: to a place a local is the root of,
  what's declared there (step 60's A6); to what a reference points to,
  that reference; to a field of what a reference a call gave points to,
  what's declared of what it's in at the field's positions, and the
  field's own carried refinements; of a temporary, which nothing else can
  name, what a `let` of it would declare; of a pattern's `&mut` binding at
  a place, what's declared there; and of what a call gives, what the
  instantiation says (step 60's A3), and through a projection, W6. No case
  falls to "nothing declared" for want of a rule: `lvalue` is a match on
  the three kinds of place expression (a variable, a field, a `*`), and
  what isn't one is a temporary.
- **Lemma W2 (writes).** A write through a reference, or to a field of
  what it points to, must have what the reference says at that position
  (A6), which W1 makes what's declared there.
- **Lemma W3 (proved again, not assumed).** While a place lent to a call
  is proved again after it (step 61's D1), what's declared of what it's in
  isn't assumed at the positions it's at: reading a tuple whose type
  refines its element would otherwise say of the element what's being
  proved of it.
- **Lemma W4 (temporaries).** A field lent straight to a call-scoped
  `&mut` keeps its refinements because they're proved again after the
  call, at its place (step 30). A temporary's field has none: so it's only
  lent so if what's said of it is carried in its type arguments and names
  no field, which W2 keeps of every write.
- **Lemma W5 (given whole).** Where a type parameter's type is invariant
  inside (behind a `&mut`, in an `Unsafe`), the callee may write values of
  the parameter's type there through what it's given (an `IterMut`'s
  items), as it may behind a `&mut` of it: so what's declared there must
  be implied by every case of the instantiation, as A3 already asked of an
  invariant occurrence. What's declared of a value (not a place) is what a
  `let` of it would declare.
- **Lemma W6 (projections).** If refinements were types, `<X as
  Tr>::A` for a refined `X` would normalise as for its base: the impl's
  `A`, its parameters what's at their positions in `X`. The impl's
  methods give values of a parameter only from what they're given, by
  parametricity (A2), unless a bound of the impl's can make one, which
  `makes` asks of the impl as of a function (a bound on a projection of
  the parameter counts, `where I::Item: Int`, whose literals make one); a
  bound that isn't a consumer can't, if the impl of it for what the
  parameter is is parametric too (by induction on how deep the impls
  go). A parameter at two positions
  in `X` that say different things, or at none, is unknown: so is the
  projection, and where a reference could be written through it, the
  call is refused rather than taken to declare nothing.
- **Lemma W7 (closures given).** A closure only returns references it
  was given (T's borrow checker refuses one to what it captured), so what
  it writes is behind its parameters. Where a callee calls a type
  parameter `F` with a `&mut` to what's declared `P`, `F` must have been
  walked with exactly `P` of that parameter: so what it writes there has
  `P`, and what it counts on of it holds. A closure written in the call,
  that the callee doesn't keep, is walked so (step 60); anything else
  (another closure, a function, a function pointer) says nothing of its
  parameters, and is refused. Every closure literal records what it was
  walked with, by its type, and every call checks it, so a closure that a
  struct keeps is checked where it's called.

The adversarial tests are `tests/t/err_soundness_element_*.t`,
`err_soundness_tuple_element_binding.t`, `err_soundness_adapter_writes.t`,
`err_soundness_adapter_makes.t`, `err_soundness_item_equality_bound.t`,
`err_soundness_closure_values.t`, `err_soundness_receiver_element.t` and
`err_soundness_temporary_field_lent.t`, and the generated family
`generated_writes_through_refined_places` in `tests/generated_semantics.rs`:
six places a type argument refines, twenty ways of getting a `&mut` to
one, each written through six ways that must keep the refinement (good)
or be refused (bad), and given four ways to code that may write anything.

Each rule was switched off in turn, and a test failed for each, but for
those that only back others up:

- W6's refusal of a projection it can't say is reached only where
  `makes` has already said its base's values are made, so W5 refuses the
  call first;
- W6's check of an impl's own bounds and `parametric_impl`'s are one
  condition, asked at two points: switched off together, a test fails;
- a parameter at two positions of an impl's type is one
  `parametric_impl` already says isn't parametric;
- what a reference a `*` of a temporary makes says is said by what the
  temporary declares too;
- W7's other direction, that what a closure counts on holds of what it's
  given: a closure walked with something declared is one written in a
  call that doesn't keep it, so it's only ever given what it was walked
  with.

They stay, so that each lemma is checked where it's used, not only where
it's first true.

## What it doesn't do

- `for r in (0..8u8).rev()` still doesn't know `r < 8`: a range's values
  are made by its `Int` arithmetic, not carried from a type argument, and
  `next_back`'s refinement is of the range it's called on, which `Rev`'s
  generic `next` can't say.
- A closure written in a call that keeps it, such as `map`'s, can't be
  given a `&mut` to what's refined at all, even if what it writes keeps
  it; nor can a closure made before the call. A closure's parameters
  can't be refined, so there's no way to say what one keeps.
- `swap` of a refined element with a refined variable can't be proved:
  `swap` ensures nothing, so the variable's refinement can't be proved
  again after the call. Two elements of lists or arrays whose types say
  the same can be swapped.
- The refinement walk still works on the syntax, not on MIR: retrospective
  6 decided on a design pass for moving it, which comes next.

## Decisions

Retrospective 6's, as recommended:

- D1, every place expression says what's declared at it, and projections
  are followed through impls (so adapters carry what a type argument says
  of their items), with whatever can't be said refused, not assumed.
- D2, a design pass for a refinement checker on MIR comes after this
  step.
- D3, then refinement aliases and refinements of variants' fields.

## Exercises

1. Find the line in `lvalue` that a fourth kind of place expression would
   have to be added to, and say what the compiler would do if it weren't.
2. Why is a temporary declared what a `let` of it would declare, and not
   nothing? Write a program where nothing would be unsound. (Hint: a
   destructor.)
3. `IterMut<'a, T>` is invariant in `T`. Give a function generic in an
   `I: Iterator` that, given two iterators of differently refined lists
   as one `I`, could write one's element into the other's, if W5 didn't
   refuse it.
4. Write an iterator adapter whose `next` makes values of `I::Item`, and
   check that `l.iter().my_adapter()`'s values say nothing.
5. W7 refuses a closure made before the call. What would a closure type
   have to carry for it to be checked instead, and where would that be
   written?

## Reading

- Nico Lehmann, Adam T. Geller, Niki Vazou and Ranjit Jhala, *Flux:
  Liquid Types for Rust* (PLDI 2023): refinements in Rust's types, checked
  on MIR, with mutable references invariant and strong updates through
  owned places.
- Philip Wadler, *Theorems for free!* (1989): what lemma W6 leans on for
  an impl's methods.
- The Rust Reference, *Place expressions and value expressions*: the
  distinction `lvalue` makes, and temporaries.
