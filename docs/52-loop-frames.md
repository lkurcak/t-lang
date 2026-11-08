# Step 52: loop frames

Code: `Walk::frames`, `Walk::changes`, `Walk::keeps` and `Walk::forget` in
`src/check/refine.rs`.

At the head of a loop the checker forgets what it knew of every place the
loop changes, since by the next time round it may have changed (step 8).
That included a place the loop only lends to calls that promise to leave
some of it alone. `List::set` ensures `self.len() == old(self.len())`, and
yet this loop couldn't be proved:

```rust
use alloc::list::List;

fn ones(l: &mut List<u8>) ensures l.len() == old(l.len()) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1); // needs `i < l.len()`: known, as `l.len()` is still `n`
        i = i + 1;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut l: List<u8> = List::filled(3, 0);
    ones(&mut l);
    out.line(&l[2])?; // expect: 1
    Ok(())
}
```

`n` is still known to be `l.len()` at the head of every iteration, and
after the loop, where `ensures` needs it. In t-libs's grid the problem was
the same, one loop deeper: the outer loop's `y < g.height()` was
forgotten at the head of the inner loop, which lends `g` to `set`, so
every inner loop wrote out the outer condition again.

## The frame of a loop

A loop's *frame* is what it keeps the same of the places it changes. The
checker works it out from the loop's body before walking it. A measure
`m` of a place `p` is in the frame if every change in the body to `p`,
to a place inside `p`, or to a place `p` is inside of, is a call that:

- lends `p` itself, straight, to a call-scoped `&mut` parameter (one no
  reference made from it outlives the call: step 30), and
- says in its postconditions that `m` is as it was.

The second point is proved once for each call, apart from the loop. The
callee's `ensures` is assumed of two new values, what's behind the
`&mut` before the call and after it, with new values for the other
arguments, and each measure it names of the value after must come out
equal to the same measure of the value before. The other arguments are
new so the proof holds for every value they take, on every iteration,
not only the first. So a call whose promise depends on its other
arguments doesn't keep the measure:

```rust,error
use alloc::list::List;

mod grow {
    use alloc::list::List;

    pub fn grow(l: &mut List<u8>, more: bool) with alloc ensures more || l.len() == old(l.len()) {
        if more {
            l.push(0);
        }
    }
}

fn f(l: &mut List<u8>) with alloc {
    let n = l.len();
    let mut more = false;
    let mut i: usize = 0;
    while i < n {
        grow::grow(l, more); // `more` is `false` only the first time round
        more = true;
        l.set(n - 1, 1);
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
```

Anything else that changes the place, in any iteration, takes the
measure out of the frame: an assignment to it, a lend of it to a call
that doesn't promise the measure (`push`, which ensures `len()` grew by
one), a lend of the struct it's a field of, or a `&mut` of it kept in a
local:

```rust,error
use alloc::list::List;

fn f(l: &mut List<u8>) {
    let n = l.len();
    let mut i: usize = 0;
    while i < n {
        l.set(i, 1);
        let r = &mut *l;
        r.clear();
        i = i + 1;
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `set`
```

At the loop's head, and after a loop left by `break`, the place still
gets a new value nothing is known of, but each measure in the frame
equals what it was before the loop.

## Why it's sound

- **Lemma F1 (measures are of the value).** A measure is a function of
  the value it's given, nothing else: its body is pure, and it can't read
  what's behind a cell (lemma K1 of docs/37-cell.md). So two values that
  a postcondition says have the same `m` have the same `m` from then on.
- **The induction.** Take a measure `m` of `p` in the frame, and any
  point of the loop's body or its head, on any iteration. Since the loop
  began, `p` changed only at calls in the frame's rule: the body has no
  other change to `p`, nor to a place around it or inside it, and `p` is
  stable (`Walk::unstable_in`): no closure captures it and no `&mut` of
  it outlives where it's made, so nothing changes it except where the
  body says. Each such call left `m` as it was, by what the callee
  proved at its return (step 30) and the check above, which holds for
  whatever the other arguments were. So `m` is what it was before the
  loop.
- **The check is about new values.** The values the postcondition is
  assumed of are new variables, so nothing known at the loop says
  anything of them, and the facts the check assumes are dropped after
  it.
- **It's only more facts at the head.** The place still has a new value
  there, as before; the frame adds equalities, each proved above. A loop
  with no frame is checked as it was.
- **The interpreter still checks.** `t run --interp` checks every
  refinement and every operator marked proved at run time.

## What it doesn't do

- A place assigned in the loop keeps nothing, even an assignment that
  leaves it as it was. Explicit loop invariants could say more; they
  aren't in T. (Since step 54 they are: docs/54-loop-invariants.md.)
- A call by path, `List::set(l, i, v)`, lends through its argument
  expression, not straight to the call, so it keeps nothing; write
  `l.set(i, v)`. (Since step 53, `List::set(&mut l, i, v)` keeps what it
  ensures: an explicit `&mut` argument lends straight to the call.)
- Inside the module that defines a measure, the measure is its body, of
  the struct's fields, and those are forgotten with the place; the frame
  helps the module's users.
- `for x in 0..n` still doesn't say `x < n`: `Range::next` gives an
  `Option<T>`, which can't. That's the next step's: refinements inside
  type arguments (done in step 53, docs/53-refined-type-arguments.md).

## Decisions

- The frame is inferred from what the calls ensure, with no new syntax,
  rather than written as loop invariants: every loop that only writes
  through such calls gets it for free.
- The next step is refinements inside type arguments (`Option<{ v: T |
  v < self.end() }>`), which is what `for` over a range needs, and what
  `Grid::new` needs to say how big the grid it gives is.

## Exercises

1. A loop calls `l.swap(i, j)` and `l.set(i, v)` on the same list. Is
   `len()` in its frame? What about a loop that also calls
   `l.truncate(l.len())`?
2. Why must the check give the other arguments new values, rather than
   their values where the loop starts? Write the program that would be
   wrongly accepted.
3. The frame is worked out before walking the body, from its syntax and
   the callees' signatures. Sketch the alternative, Houdini-style: assume
   every equality at the head, walk the body, drop those not proved at
   the end, and repeat. What would it accept that this doesn't, and what
   would it cost?

## Reading

- Leino, *Dafny: An Automatic Program Verifier for Functional
  Correctness* (2010): loops `modifies` frames, and how Dafny infers
  that a loop doesn't change what it doesn't write.
- Flanagan and Leino, *Houdini, an Annotation Assistant for ESC/Java*
  (2001): inferring loop invariants by dropping candidates that don't
  hold.
- Reynolds, *Separation Logic: A Logic for Shared Mutable Data
  Structures* (2002): the frame rule, which this is a small case of.
