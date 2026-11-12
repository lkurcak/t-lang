# Step 69: what a loan forgets

Code: `Walk::mir_borrow`, `Walk::bind_ref`, `Walk::lent_place`,
`Walk::changes_in` and `Walk::closure_frames` in `src/check/refine/mir.rs`;
`Walk::frames_of`, `Walk::measures_now` and `Walk::measures_kept` in
`src/check/refine.rs`.

Since step 8, a variable a closure captured, if it was `mut` or held a
`&mut`, and a place a pattern bound something in by `&mut`, gave a new
value each time it was read, anywhere in the function: before the
closure was made, and long after it was gone (the walk called them
*unstable*). So none of these could be proved:

```rust
/// Read before the closure is made.
fn before() -> { v: i64 | v == 5 } {
    let mut x: i64 = 5;
    let y = x;
    let mut inc = || x = x + 1;
    inc();
    y
}

/// Assigned after it's gone.
fn after() -> { v: i64 | v == 5 } {
    let mut x: i64 = 0;
    let mut inc = || x = x + 1;
    inc();
    x = 5;
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&before())?; // expect: 5
    out.line(&after())?;  // expect: 5
    Ok(())
}
```

Now a closure's capture, and a pattern's binding, forget the place where
they're made, and only there, as an explicit `&mut` always has.

## A loan

A place changes, other than by an assignment to it, only through a
`&mut` of it, a *loan*. On MIR (docs/65-refinements-on-mir.md) each loan
is a borrow, `Ref(_, mut, q)`:

- an explicit `&mut q`, or a method's `&mut self`;
- a closure's capture of a variable it changes, or changes what's behind
  (it captures the variable `&mut`, or uniquely);
- a `move` closure's capture of a `&mut`: any use of a `&mut` by value is
  a new borrow of what it points to, `&mut *r`;
- a pattern's binding by `&mut` (match ergonomics), of a field, a
  variant's field or an array's element.

While a loan lives, the borrow checker lets nothing read or write the
place, or a place in it, but through the loan. So the walk forgets what
it knew of the place where the loan is made, and the next time it reads
the place, the loan is gone: the value is one nothing's known of but
what's declared there, and from then on the place is known as any other.
A `&mut` given straight to a call is still forgotten after the call
returns, when what the callee ensures is learned (step 30).

> **Lemma S1 (loans are the only other way).** Of MIR's statements,
> `Assign` writes the place it names, and `Ref(_, mut, q)` makes the only
> kind of value through which `q` can be written (lemma M2 of
> docs/65-refinements-on-mir.md; a `&` can't write, but in a `Cell`,
> whose contents the walk never knows). So each change to a place the
> walk knows of is an assignment it sees, or a write through a loan of
> the place, or of one it's in.

> **Lemma S2 (forgetting at the start is forgetting at the end).** Let a
> loan of `p` be made at point `a` and last used at point `b`. The
> borrow checker accepts the function only if no statement between `a`
> and `b` reads or writes `p`, or a place in it or around it, other than
> through the loan (its loans are non-lexical: a loan lives exactly
> where it may still be used). The walk forgets `p` at `a`, and so reads
> nothing of it before `b`; its first read after `b` gives a new value
> that has only what's declared there. That's the value Flux gives a
> *blocked* place when its loan ends: the same knowledge, without the
> walk needing the borrow checker's regions.

> **Lemma S3 (closures' bodies).** A closure's body is walked from what's
> known where it's made, or at the call it's given to. What it captured
> `&mut` was forgotten by the capture's loan, before either, so each run
> of the body reads it as a new value; what it captured by `&` can't
> change while it lives, so what's known of it where it's made holds at
> every run; what it captured by value is the value it was made with.

> **Lemma S4 (parts the walk doesn't name).** The walk names places
> through fields and `*`s, but not a variant's field or an array's
> element. A loan of one forgets the place it's in that the walk does
> name (`lent_place`): what it knew of everything in it. In a loop, the
> loan is a change to that place too.

S4 matters: with the old rule switched off, a binding into `Some(n)`
inside a loop was a loan the loop's changes didn't see, as the walk
couldn't name its place, and this was accepted:

```rust,error
fn f(o: &mut Option<i64>) -> i64 with {} {
    *o = Some(1);
    let mut k: i64 = 0;
    while k < 2 {
        match *o {
            Some(v) => {
                let _w = 10 / v;
            },
            None => {},
        }
        match o {
            Some(n) => {
                *n = 0;
            },
            None => {},
        }
        k = k + 1;
    }
    0
}

fn main() {}
// error: `/` may panic, but `f` doesn't say `with panic`
```

The old rule covered it, by making `*o` unknown everywhere; now it's a
loan of `*o`, forgotten in the loop as any other change.

A closure that borrows a variable by `&` sees what's known of it where
it's made, even when it's run later, by a function it's given to:

```rust
fn apply<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 { f() }

fn frozen() -> i64 with {} {
    let mut d: i64 = 2;
    let c = || 10 / d;
    let r = apply(c);
    d = 0;
    r + d
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&frozen())?; // expect: 5
    Ok(())
}
```

`d` can't be assigned while `c` has it borrowed, so `d` is 2 at every run
of `c`. A `move` closure's `d` is its own copy, of the value `d` had when
it was made; what `d` is at the call doesn't matter:

```rust,error
fn apply<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut d = 0;
    let c = move || 10 / d;
    d = 1;
    out.line(&apply(c))?;
    out.line(&d)?;
    Ok(())
}
// error: this closure may panic, but `apply` needs calls of it to fit `fn() -> i64`, which can't panic
```

## Closure frames

A loop keeps the measures its body's calls keep (docs/52-loop-frames.md).
A closure's body may run many times too, while it has what it captured,
and the same rule says what it keeps. t-libs's hash map grows by moving
each entry into new buckets with a closure that captures `self` and
calls `put`, which ensures `len()` stays; so `grow` can say so too:

```rust
use alloc::list::List;

#[derive(Discard)]
struct M { buckets: List<List<u64>>, len: usize }

impl M {
    #[measure]
    fn len(&self) -> usize { self.len }

    /// Puts `e` in a bucket of its own; `len` is the caller's.
    fn put(&mut self, e: u64) with alloc ensures self.len() == old(self.len()) {
        let mut b = List::new();
        b.push(e);
        self.buckets.push(b);
    }

    fn grow(&mut self) with alloc ensures self.len() == old(self.len()) {
        let old = core::mem::replace(&mut self.buckets, List::new());
        old.free_with(|b: List<u64>| b.free_with(|e: u64| self.put(e)));
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut m = M { buckets: List::new(), len: 0 };
    m.put(7);
    m.len = 1;
    m.grow();
    let l: { l: usize | l == 1 } = m.len();
    out.line(&l)?; // expect: 1
    Ok(())
}
```

At a capture of `q` by `&mut`, the walk finds the closure's *frame*: of
each place in `q` its body changes, the measures every change to it in
the body keeps (`frames_of`, over the body's blocks, as over a loop's).
A change is an assignment, which keeps nothing; a `&mut` lent straight
to a call, which keeps what the call ensures of it; or a capture by a
closure inside, which keeps that closure's frame. Then each measure in
the frame is worked out, `q` is forgotten, and each is what it was.

> **Lemma F2 (a closure keeps its frame).** Let closure `c` capture `q`
> by `&mut`, and `m` of place `f` be in its frame. While `c`'s loan of
> `q` lives, only runs of `c` change `f` (lemma S2): by this function, or
> by any it's given `c`, never two at once, as a run has `c` borrowed
> `&mut`, or owns it. Each run changes `f`, places in it and places it's
> in only by changes that keep `m` of `f`: calls, by what their callees
> proved at return, and closures inside, by this lemma for them (by
> induction on how deep they are). A measure is of the value alone
> (lemma F1 of docs/52-loop-frames.md). So, by induction on the runs,
> when the loan ends `m` of `f` is what it was when it was made.

A closure that also changes the place another way keeps nothing of it:

```rust,error
use alloc::list::List;

fn f(l: &mut List<u8>) with alloc ensures l.len() == old(l.len()) {
    let mut p = |v: u8| l.push(v);
    p(1);
}

fn main() {}
// error: can't prove `l.len() == old(l.len())` for what `f` ensures
```

A change keeps measures *of places*: a loan of `d` to a closure whose
frame keeps `n()` of `d.x` says nothing of `d.y`, even if the closure
writes `d.y`. So what a loop's changes keep is now, for each, a list of
places with their measures (`Keeps`), and a measure of `p` is in the
loop's frame if every change to `p`, or to a place in it or around it,
keeps it of `p` itself. A loop that makes a closure keeping `n()` of
`*c` keeps it at its head; one that also lends `c` to a call that keeps
nothing doesn't.

## Why it's sound

- S1 to S4 replace the old rule: a place is forgotten wherever it may
  change, and at no other point the walk reads it. Nothing else changed
  in what's assumed or proved.
- F2, for what a closure's loan keeps, is F1 and the induction of
  docs/52-loop-frames.md over the runs of the closure rather than the
  times round a loop; the borrow checker gives the "nothing else changes
  it" that a loop gets from seeing all its body.
- `tests/generated_loans.rs` tries each loan (an explicit `&mut`, a call,
  a capture by `&mut`, by `move`, given to a function, nested, a
  pattern's binding of a field, a variant's field or an array's element,
  and a closure that keeps `n()` but writes it too) at each site
  (straight after, at the head of a loop that makes it, in `ensures`),
  writing a value that breaks the read or keeps it: every program is
  refused or runs without breaking a refinement, and each breaking one is
  refused for the read. It checks that what's read before the loan, and
  assigned after it, are known, and that closures keep their frames,
  called, given to a function, `move`, nested, in a loop.
- Each rule switched off on purpose makes a test fail.

## What it doesn't do

- After a `&mut` binding into a variant's field, the walk forgets the
  enum whole, even which variant it is, though a write to a variant's
  field can't change that (lemma V1 of docs/66-one-walk.md).
- A closure's body doesn't assume its frame: a run reads what it
  captured as a value nothing's known of, even the measures its loan
  keeps.
- A pattern's `&mut` binding has no frame: what's written through it
  forgets the place, even where every write keeps a measure.
- A closure ensures nothing else (docs/30-postconditions.md): t-libs's
  heap still can't say how long `into_sorted`'s list is, as its closure
  pushes, and nothing says `free_with` never runs it on an empty heap.

## Decisions

- Forget at the loan, rather than at its end as Flux does: the same
  knowledge (S2), with no regions in the refinement checker.
- Closure frames in this step, inferred as loop frames are, with no new
  syntax.
- Keeping which variant an enum is across a binding into its field:
  later.
- Released as 0.84.0: more programs are accepted, and a `with panic` that
  was needed may now be refused as unused.

## Exercises

1. `let r = &mut x; let mut c = || *r += 1; c();`. Which statements
   forget `x`, and which forget `*r`? What does the walk know of `x`
   after `c`'s last call, and why may it read `x` there at all?
2. Make `Keeps` a plain list of measures again, applied to every place in
   the capture, and write the program that's then wrongly accepted.
   (`tests/t/err_soundness_closure_frame_disjoint.t` is one.)
3. A closure captures `v` by `&` and is given to a function that runs it
   many times, and stores it nowhere. Why can its body use what's known
   of `v` where it's made? What if `v` were a `Cell`?
4. Sketch keeping the variant across a binding into its field: what's
   kept, and where's the new object for the enum made?

## Reading

- Lehmann, Geller, Vazou and Jhala, *Flux: Liquid Types for Rust*
  (PLDI 2023): strong references, and places blocked while lent.
- Matsakis, RFC 2094, *Non-lexical lifetimes* (2017): where a loan is
  live.
- Weiss, Gierczak, Patterson and Ahmed, *Oxide: The Essence of Rust*
  (2019): loans as sets of places.
- Leino, *Dafny: An Automatic Program Verifier for Functional
  Correctness* (2010), and Reynolds, *Separation Logic* (2002): frames.
