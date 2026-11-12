# Step 54: loop invariants

Code: `Checker::loop_invariant` and `Walk::loop_head`, `Walk::again` and
the `While`, `Loop`, `Break` and `Continue` cases of `Walk::expr_here` in
`src/check/refine.rs`; `Builder::holds` in `src/borrowck/build.rs`;
`StmtKind::Holds` in `src/mir.rs` and `Interp::holds`.

At the head of a loop the checker forgets what it knew of every place the
loop changes (step 8), keeping only what its frame says (step 52). A
variable assigned in the loop kept nothing, and so the most familiar
proved loop there is couldn't be written:

```rust,error
fn find(a: &[u32], x: u32) -> Option<{ i: usize | i < a.len() }> with {} {
    let mut lo: usize = 0;
    let mut hi: usize = a.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if a[mid] == x {
            return Some(mid);
        }
        if a[mid] < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    None
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

`hi` is `a.len()` before the loop, and only ever made smaller, but at the
head it's a value nothing's known of. Now a loop can say what stays true
each time round:

```rust
fn find(a: &[u32], x: u32) -> Option<{ i: usize | i < a.len() }> with {} {
    let mut lo: usize = 0;
    let mut hi: usize = a.len();
    while lo < hi invariant lo <= hi && hi <= a.len() {
        let mid = lo + (hi - lo) / 2;
        if a[mid] == x {
            return Some(mid);
        }
        if a[mid] < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    None
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let sorted: [u32; 5] = [1, 3, 5, 7, 9];
    let a = sorted.as_slice();
    if let Some(i) = find(a, 7) {
        out.line(&i)?; // expect: 3
    }
    out.line(&find(a, 4).is_some())?; // expect: false
    Ok(())
}
```

## An invariant

`invariant p` goes between a loop's head and its body, on any of the
four: `while c invariant p { .. }`, `loop invariant p { .. }`, `for x in
e invariant p { .. }` and `while let q = e invariant p { .. }`.
`invariant` is a keyword, as `ensures` is. There's one predicate; `&&`
joins several.

`p` is a predicate in the language of refinements (docs/08 and docs/21):
literals, names, measures `l.len()`, consts, `+`, `-`, `*`, comparisons,
`&&`, `||` and `!`. It names the variables in scope at the loop,
`mut` ones too, which a refinement can't: it's about their values each
time the loop is about to go round, not about one value for ever. It
can't use `old(..)`, which is a postcondition's.

In a `for` or `while let`, the pattern's variables aren't in scope: the
head is before each next value is bound. A loop that needs to say how far
it's got keeps its own count, as `filled` does here, which is how long
the list it fills is:

```rust
use alloc::list::List;

fn filled(n: usize) -> { l: List<usize> | l.len() == n } with alloc {
    let mut l: List<usize> = List::new();
    let mut i: usize = 0;
    while i < n invariant l.len() == i && i <= n {
        l.push(i);
        i = i + 1;
    }
    l
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l = filled(4);
    out.line(&l[3])?; // expect: 3
    Ok(())
}
```

`push` ensures the list is one longer than it was, which is what makes
`l.len() == i` hold again at the end of each time round.

## How it's checked

It's Hoare's rule for loops, with what step 52 keeps:

1. As the loop starts, `p` must hold.
2. At the head, every place the loop may change is a value nothing's
   known of but its refinements and its frame, as before, and `p` is
   assumed of them. A `while`'s condition is then assumed in its body.
3. The body is checked from there once, and wherever it goes round
   again, `p` must hold: at its end, and at each `continue` for the loop,
   from an inner loop too (`continue 'outer`).

```rust,error
fn f(c: bool) -> usize with {} {
    let mut i: usize = 0;
    while i < 3 invariant i < 4 {
        if c {
            i = 7;
            continue;
        }
        i = i + 1;
    }
    i
}
// error: can't prove `i < 4` for the loop's invariant, at this `continue`
```

An inner loop that changes a variable an outer loop's invariant is about
forgets it at its own head, like any other; its own invariant says what
it keeps.

## After a loop

A loop ends at one of its exits: a `while`'s condition false at the head,
or a `break` for it. Until now a loop that had a `break` forgot, after
it, everything it changed. Now each `break` is a branch: the checker
keeps how things were there, and after the loop it knows that one of the
exits was taken, as after an `if` or a `match`. So a `loop`'s value is
one of what its `break`s give, and what was true at every `break` is true
after:

```rust
fn count_to(n: { n: usize | n <= 100 }) -> { v: usize | v == n } with {} {
    let mut i: usize = 0;
    loop invariant i <= n {
        if i == n {
            break;
        }
        i = i + 1;
    }
    i
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&count_to(7))?; // expect: 7
    Ok(())
}
```

A `for` loop ends at the `break` its rewrite makes at the head when the
iterator is done (docs/42), where the invariant holds, so it holds after
a `for` that has no `break` of its own that breaks it.

## What it reads

An invariant is never run: compiled code doesn't evaluate it. It reads
the variables it names at the head, though, for two checkers:

- The borrow checker sees each one read there, each time round: an
  invariant can't name a variable that's been moved out of, or one lent
  `&mut` to something still alive at the head.
- The interpreter, `t run --interp`, checks that it holds there, each time
  round, as it checks every refinement: if it ever doesn't, the checker
  was wrong, and it says so.

```rust,error
use alloc::list::List;

fn f(n: usize) with alloc {
    let mut l: List<u8> = List::new();
    let r = &mut l;
    let mut i: usize = 0;
    while i < n invariant l.len() >= 0 {
        r.push(0u8);
        i = i + 1;
    }
    r.push(1u8);
}
// error: cannot use `l` while `l` is mutably borrowed
```

## Why it's sound

- **Lemma I1 (the head).** Every time the program gets to a loop's head,
  `p` holds. As the loop starts, it was proved. Each other time comes
  from the end of the body or a `continue`, where it was proved, in a
  state the checker's facts describe if the head's did. They do by
  induction on how many times round the loop has been: at the head every
  place the loop may change (assigned, lent `&mut`, or captured by a
  closure, as `Walk::assigned` and `Walk::unstable_in` find; since step
  69, each a loan made in the loop, docs/69-loans.md) is a new
  value, so the head's facts are of any iteration's values: the
  refinements of their types, the frame (lemma F1 of
  docs/52-loop-frames.md), which holds of every iteration, and `p`, by
  this induction. What the loop doesn't change is what it was.
- **Lemma I2 (the exits).** A loop is left only by a `while`'s condition
  being false at the head or by a `break` for it: a `return` or a panic
  leaves the function, and a closure's body can't `break` a loop around
  it. So what holds after the loop is what held at one of those, which is
  what the join of branches says (docs/08), with each exit's facts since
  the head, where they all start.
- **Lemma I3 (it's never run).** Compiled code is the same with or
  without the invariant. The borrow checker sees what it names read at
  the head, where the interpreter reads it, so the interpreter only reads
  what's there and not lent `&mut`.
- **The interpreter still checks.** At each head it checks the
  invariant, and everywhere else every refinement, as before.

## What it doesn't do

- Nothing's inferred: an invariant says only what it's written to say,
  besides the frame (step 52), which is still inferred.
- A `for`'s or `while let`'s invariant can't name the pattern's
  variables, so `for x in a..b` still doesn't know `a <= x`
  (docs/53-refined-type-arguments.md); a `while` with its own counter
  can say it.
- Nothing says a loop ends. A `with {}` function's loop may still run for
  ever; T has no `div` effect yet (step 7).
- `continue` exits aren't joined; nothing needs them to be, as they go to
  the head, where the invariant is all that's assumed.

## Decisions

- D1, syntax: `invariant p` after the loop's head, a keyword, with one
  predicate. Rather than a list of predicates (Verus) or an attribute.
- D2, which loops: all four. The pattern's variables aren't in scope in
  a `for` or `while let`.
- D3, a loop's exits are joined as branches are, rather than forgetting
  after a loop with a `break` what it changed.
- D4, an invariant reads what it names at the head, for the borrow
  checker: it can't name a moved variable, or one lent `&mut` across the
  head, even where the predicate would hold anyway.

## Exercises

1. Write `rev(a: &mut [u8])` with a `while` that swaps `a[i]` and
   `a[n - 1 - i]`. Does it need an invariant? Why does the binary search?
2. `while i < n invariant i <= n { i = i + 2; }` fails. Why? What does
   `i <= n` need to be, or the loop to do, to be proved?
3. Give a loop whose invariant holds at every `break` but not after the
   loop, if the exits weren't joined but the invariant were assumed after
   any loop. Then one where joining keeps a fact the invariant doesn't
   say.
4. Houdini (below) infers invariants: assume each candidate fact at the
   head, drop those not proved at the back edges, and repeat. Which
   candidates would find `lo <= hi && hi <= a.len()` for `find`?

## Reading

- Floyd, *Assigning Meanings to Programs* (1967), and Hoare, *An Axiomatic
  Basis for Computer Programming* (1969): the loop rule.
- Leino, *Dafny: An Automatic Program Verifier for Functional
  Correctness* (2010): `invariant` clauses on loops, checked the same way.
- Lattuada et al., *Verus: Verifying Rust Programs using Linear Ghost
  Types* (2023): loop invariants in a Rust-like language, ghost code the
  compiler drops.
- Flanagan and Leino, *Houdini, an Annotation Assistant for ESC/Java*
  (2001): inferring them.
