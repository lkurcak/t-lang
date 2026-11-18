# Step 84: counting in loops

Code: the name in `Parser::loop_expr` (`src/parser.rs`) and
`modules::Rewrite::for_loop`; `len` and the `next`/`next_back` of
`Iter`, `IterMut` and `Copied` in `core/slice.t`, of `IntoIter` in
`core/array.t`; `Range`'s `next`/`next_back` in `core/ops.t`; `swap` in
`Walk::mir_call` (`src/check/refine/mir.rs`), until step 85 gave it a
signature (docs/85-values-in-predicates.md).

A count kept in a `for` couldn't be proved not to overflow. t-chess
counts how many times a position has stood on the board, and wrote
`saturating_add`, which never saturates there:

```rust,error
use alloc::list::List;

fn repetitions(history: &List<u64>, g: u64) -> usize with {} {
    let mut count: usize = 0;
    for old in history.iter() {
        if *old == g {
            count = count + 1;
        }
    }
    count
}
// error: `+` may panic, but `repetitions` doesn't say `with panic`
```

A count of a list's values can't be more than its length, but nothing
said so. A `for`'s invariant (docs/54) is checked at the loop's head,
before the pattern's variables are bound, so it couldn't name them. It
couldn't name the iterator either, which the `for` keeps under a name no
program can write. And no iterator said how much it had left. A `while`
with an index of its own could say `n <= i`, which is what t-life's
`population` does.

## A `for` names its iterator

`for x in it: e` names the iterator the `for` goes through `it`. It's in
scope in the invariant and in the body, where it's a `mut` variable like
any other, and not in `e`, which is what makes the iterator. A `for` was
already a rewrite (docs/06e):

```text
match core::iter::IntoIterator::into_iter(e) {
    mut it => loop invariant p {
        match core::iter::Iterator::next(&mut it) {
            Some(x) => body,
            None => break,
        }
    },
}
```

and the name is the one that rewrite binds. So naming it adds no rule:
what a `for` that names its iterator means is what the `while let`
written with that variable meant (lemma N1). The body can call `it.next()`
to skip a value, or read what's left. It can even give `it` a new value,
or lend it to a function, and then the invariant is checked of that, as
of any variable it names.

The syntax is Verus's. No expression starts with a name and one `:`, so
`it:` can only be a name.

## What's left

Every iterator over a slice, an array or a list now says how much it has
left, as a measure `len()`: `Iter`, `IterMut` and `Copied` of a slice,
`IntoIter` of an array, and `IntoIter` of a list, which said it since
step 80. Each `next` and `next_back` says what list's `IntoIter::next`
did:

```text
fn next(&mut self) -> { r: Option<..> | r.is_some() == (old(self.len()) > 0) }
ensures
    self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0),
```

It gives a value while it has any, and each value takes one off what's
left. What makes one, `iter`, `iter_mut`, `copied`, `into_iter` and a
`for`'s `&l`, `&mut a` or `a`, says it starts with as many as there are.

So a count kept with what's left adds up to no more than the length:

```rust
use alloc::list::List;

fn repetitions(history: &List<u64>, g: u64) -> { n: usize | n <= history.len() } with {} {
    let mut count: usize = 0;
    for old in it: history.iter() invariant count + it.len() <= history.len() {
        if *old == g {
            count = count + 1;
        }
    }
    count
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut h: List<u64> = List::new();
    h.push(3);
    h.push(4);
    h.push(3);
    out.line(&repetitions(&h, 3))?; // expect: 2
    Ok(())
}
```

At the head, `count + it.len() <= history.len()` is assumed. In the body,
`next` gave a value, so `it.len()` is one less than it was there: `count`
is less than `history.len()`, and `count + 1` can't overflow. After the
body the invariant holds again, and after the loop it says `count <=
history.len()`.

An index kept the same way, `i + it.len() == a.len()`, is less than
`a.len()` in the body, so it can index another slice as long. That's the
work `enumerate` does, without the `panic` its counter needs (see below):

```rust
fn same(a: &[u64], b: { b: &[u64] | b.len() == a.len() }) -> { n: usize | n <= a.len() } with {} {
    let mut i: usize = 0;
    let mut n: usize = 0;
    for x in it: a invariant i + it.len() == a.len() && n <= i {
        if *x == b[i] {
            n = n + 1;
        }
        i = i + 1;
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&same(&[1, 2, 3], &[1, 0, 3]))?; // expect: 2
    Ok(())
}
```

## Ranges step by one

A range's iterator says where it's up to, `start()`, and since step 82
that `next` never takes it back. Now it also says by how much: one, when
it gives a value, and none, when it doesn't:

```text
ensures
    (self.start() == old(self.start()) + 1 && old(self.start()) < self.end())
        || (self.start() == old(self.start()) && old(self.start()) >= self.end()),
```

and `next_back` the same of `end()`. So a count is at most where the
range is up to:

```rust
fn ones(a: &[u8; 64]) -> { m: u8 | m <= 64 } with {} {
    let mut m: u8 = 0;
    for s in it: 0..64u8 invariant m <= it.start() && it.start() <= 64 {
        if a[s as usize] == 1 {
            m = m + 1;
        }
    }
    m
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = [0u8; 64];
    a[3] = 1;
    a[60] = 1;
    out.line(&ones(&a))?; // expect: 2
    Ok(())
}
```

In the body, `m <= it.start()` is enough: the value `next` gave was below
the end, which `next` keeps (the loop's frame, docs/52, so `it.end() ==
64` needn't be said), so `m + 1` is at most 64. `it.start() <= 64` is
there for after the loop: a range needn't start below its end (`9..3` is
empty, and starts at 9), so its end isn't a bound on its start.

## `swap` exchanges

`IterMut::next` takes what's left out of the iterator, as a
`&mut [T]` can't be copied, cuts its first value off, and puts the rest
back. Taking it out was `mem::replace`, whose signature says it gives back
a `T`, not which one: so how long it was was lost. It's `mem::swap` now,
and the refinement checker knows what `swap` does: it exchanges what its
two places hold. So what was known of each is known of the other after,
here and in any program:

```rust
use alloc::list::List;
use core::mem::swap;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut a: List<u8> = List::new();
    a.push(7);
    let mut b: List<u8> = List::new();
    swap(&mut a, &mut b);
    out.line(&b[0])?; // expect: 7
    Ok(())
}
```

`a[0]` there is refused: `a` is the empty one now
(`err_soundness_swap_exchanged.t`).

Since step 85 it's no special case: `swap`'s signature says it, `ensures
*a == old(*b), *b == old(*a)`, and `replace`'s says which value it gives
back (docs/85-values-in-predicates.md).

## Why it's sound

- **N1 (a name, no rule).** `for x in it: e invariant p { body }` is
  `match into_iter(e) { mut it => loop invariant p { match
  it.next() { Some(x) => body, None => break } } }`, which is checked by
  the rules for any `match`, `loop` and variable: docs/54's loop rule
  (an invariant holds as the loop starts, is assumed at its head, and is
  proved wherever it goes round), and the borrow checker's reading of what
  it names at the head. The `while let` with a variable of the program's
  own, `let mut it = e; while let Some(x) = it.next() invariant p
  { body }`, was checked by the same rules. Only the messages (which say
  "the `for`") and the editors (which see where the name is written,
  and not the rewrite's uses of it) know the difference.
- **N2 (what it gives).** What an iterator gives is kept by a call of its
  protocol only, and an assignment, a lend or a write through a
  reference makes it another iterator, which gives what that one gives
  (docs/82, "What keeps it"): the same of a named `for`'s iterator as of
  a variable (`err_soundness_for_name_items.t`). A `for` that runs to its
  end uses its iterator up (W5 of docs/80): its `None` came from `next`
  on whatever `it` held then.
- **L1 (what's left).** Each new `ensures`, result refinement and
  measure is of code in `core` or `alloc`, proved when the library is
  checked, as any function's are. `len` is a measure of the iterator's
  fields, which are private, and of the length of a slice, which a
  reference to it can't change. No intrinsic's signature changed.
- **R1 (ranges).** The new clause is proved of `next`'s body, as the
  others are. Step 82's bounds on a range's states (I2 of docs/82) read
  the clauses that move a measure one way, which are still there.
- **M1 (swap).** `swap(a, b)` is an intrinsic that exchanges the values
  `*a` and `*b`, making and throwing away none. The borrow checker keeps
  the two places apart, as both are lent `&mut` at once, so neither is
  part of the other, and nothing else reaches them during the call. So
  after the call `*a` holds what `*b` held before it, and the other way
  round: the walk writes each place's value to the other, as two moves
  would. What's declared of each place was checked of what's written to
  it by the call's own instantiation (docs/74), which is the same as
  before.

## What it doesn't do

- An adapter has no `len`: `rev`, `map`, `filter`, `enumerate`, `skip`
  and `zip` are generic in their iterator, and a measure can't be a
  trait's method yet, which `len` would have to be for `Rev<I>` to
  say it of `I`. So `Iterator::count`, and `enumerate`'s counter, still
  say `with panic`.
- The iterators of a `str` (`chars`, `split`, `lines`) say nothing of what
  they have left.
- `mem::replace` still gives back a `T` it says nothing of. It's
  `swap` that the walk knows, and a function's callers only see its
  signature. (Step 85: no longer; `replace` says it gives back
  `old(*dest)`, docs/85-values-in-predicates.md.)
- The pattern's variables still aren't in the invariant's scope: it's
  checked before they're bound.
- A `for` over values that aren't `Discard` (docs/80) can stop early once
  its body has used up what's left, `while let Some(x) = it.next() {..}`
  and then `break`. Moving `it` instead, into a `for` of its own, isn't
  understood: where the loops' ends meet, `it` is moved on one path and
  empty on the other, and the check of what's dropped there doesn't know
  which (`while let` over a variable was refused the same way before).

## Decisions

- **D1: `for x in it: e`**, Verus's syntax: not an invariant with a
  binder of its own (`invariant |it| p`), which reads like a closure and
  couldn't reach the body, and not only `while let` (no new syntax),
  which loses what a `for` does: its head's temporaries (docs/42), `for`
  over collections (docs/73), and linear iterators (docs/80).
- **D2: in the invariant and the body.** A variable like any other, which
  the body can call `next` on: no new rule, rather than a name only the
  invariant sees.
- `len`, as Rust's `ExactSizeIterator::len`, and as list's `IntoIter`
  already said. The range's step is said in `ensures`, which the
  formatter can lay out, rather than in the result's refinement.
- `swap` in the walk, rather than in a signature: what it says, "the
  other's value", is no predicate a signature can write. (Until step 85,
  which made it one: `*a == old(*b)`, docs/85-values-in-predicates.md.)

## Exercises

1. Write `count_if(a: &[u64], f: ..)` counting the values `f` is true of,
   with a result no more than `a.len()`. Then write it over
   `a.iter().filter(..)`: what can't you say, and why?
2. `for x in it: a invariant n + it.len() <= a.len()` with `it.next()` in
   the body still holds. Why? With `i + it.len() == a.len()` it doesn't
   (`err_soundness_for_name_skip.t`): what would `b[i]` read?
3. Take `self.len() + 1 == old(self.len())` out of `Iter::next`'s
   `ensures`. Which of this step's tests fail, and which still pass?
4. `swap(&mut x, &mut y)` with `x: { v: u8 | v < 8 }` and `y: u8`: what
   must hold of `y` first, and why? (docs/74)
5. Write `IterMut::next` with `mem::replace`. Which of its `ensures`
   can't be proved, and what would `replace` need to say?

## Reading

- Hoare, *An Axiomatic Basis for Computer Programming* (1969): the loop
  rule, which a `for` is now one more instance of.
- Verus, *for loops* (Verus guide): `for x in it: e`, a loop naming its
  iterator for its invariants.
- Denis, Jourdan and Marché, *Creusot: a Foundry for the Deductive
  Verification of Rust Programs* (2022): a `for`'s invariant over what its
  iterator has produced so far, `produced`, the general form of `len()`.
- Rust's `ExactSizeIterator`: `len()`, what an iterator has left, which
  slice iterators have.
- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983): why
  a `swap` generic in `T` can only exchange what it's given.
