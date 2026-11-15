# Step 80: values that may be dropped when a refinement holds

Code: `impl Discard for { v: T | p }` rewritten as an impl of
`core::mem::DiscardWhen` (`modules::Rewrite::discard_when`,
`src/check/modules.rs`); the drops that need `p`, found by
`borrowck::moves::linear` (`src/borrowck/moves.rs`, `Notes::drops_when`)
and proved by `Walk::mir_drop_when` (`src/check/refine/mir.rs`); the drop
glue that calls its `discard` (`Cx::drop_body`, `src/mono/glue.rs`); and
the two impls in `alloc/list.t`.

A value that isn't `Discard` must be used up: passed on, returned, or
taken apart (docs/06a.1). A list of them is one too, as dropping it would
drop what it holds. Until now that held even of a list that holds
nothing, and it made two things impossible that retrospective 10 found
programs wanting:

```text
fn close_all(fds: List<Fd>) {
    for fd in fds {
        close(fd);
    }
    // error, on 0.94: a `for` throws its iterator away when it ends, and
    // `IntoIter<Fd>` isn't `Discard`
}

fn sorted(mut h: Heap<Fd>) -> List<Fd> with alloc {
    let mut out = List::new();
    while let Some(fd) = h.pop() {
        out.push(fd);
    }
    out
    // error, on 0.94: `h` is never used up, and its type, `Heap<Fd>`,
    // isn't `Discard`
}
```

Both throw away a value that holds nothing. A `for` that runs to its end
leaves its iterator empty, and a heap `pop` says is empty is empty: the
checker knew both, as refinements (docs/21, docs/30), but `Discard` was
only ever about types. Programs used `free_with`, which gives each value
to a closure: t-life joined its threads with `handles.free_with(|h|
h.join())`, and t-libs' heap emptied a heap it knew was empty with one.

## Discard, where a refinement holds

A type can say when its values hold nothing that must be used up, with an
impl of `Discard` for a *refined* type:

```rust
use alloc::list::List;

struct Fd { n: i64 }

struct Bag { items: List<Fd> }

impl Bag {
    #[measure]
    fn len(&self) -> usize { self.items.len() }
}

/// One that holds nothing.
impl Discard for { b: Bag | b.len() == 0 } {
    fn discard(self) {
        let Bag { items: _empty } = self;
    }
}

fn main() with alloc {
    let mut b = Bag { items: List::new() };
    b.items.push(Fd { n: 1 });
    if let Some(fd) = b.items.pop() {
        let Fd { n: _ } = fd;
    }
}
```

`b` goes out of scope, and `Bag` isn't `Discard`. But the impl says a
`Bag` whose `len()` is `0` may be thrown away, and the checker proves `b`
is one there: `push` made it `1` long, and `pop` took that one out
(docs/30). So it's dropped, by the impl's `discard`, which is checked
knowing `self.len() == 0`: it drops `items`, a `List<Fd>`, which `alloc`
says the same of, and which is empty, as `self.len()` is
`self.items.len()`.

Where it can't be proved, it's an error, which says what was needed:

```rust,error
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
}
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
```

`alloc` has two such impls: for an empty `List<T>`, and for a list's
`IntoIter<T>` that's given all its values, which has a new measure,
`len()`, of how many it has left. Its `next` says what it gave:

```text
fn next(&mut self) -> { r: Option<T> | r.is_some() == (old(self.len()) > 0) }
ensures
    self.len() + 1 == old(self.len()) || (self.len() == 0 && old(self.len()) == 0),
```

So once it gives `None`, `len()` is `0`. And `into_iter` says the
iterator has as many as the list had, so that of an empty list may be
dropped at once. A `for` is a `loop` that calls
`next` until it gives `None`, and then drops its iterator (docs/42), which
is now proved empty there:

```rust
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut fds: List<Fd> = List::new();
    fds.push(Fd { n: 3 });
    fds.push(Fd { n: 4 });
    let mut total = 0;
    for fd in fds {
        total = total + close(fd);
    }
    out.line(&total) // expect: 7
}
```

A `for` that stops early leaves values in its iterator, and can't throw
them away: a `break`, a `return`, or a `?` in its body is an error.

```rust,error
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() with alloc {
    let mut fds: List<Fd> = List::new();
    fds.push(Fd { n: 1 });
    for fd in fds {
        if close(fd) == 1 {
            break;
        }
    }
}
// error: the `for` may stop before its iterator is used up, and drops it then: `IntoIter<Fd>` isn't `Discard`, only `{ it: IntoIter<Fd> | it.len() == 0 }` is
```

To stop early, hold the iterator, and use up what's left after:

```rust
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut fds: List<Fd> = List::new();
    fds.push(Fd { n: 1 });
    fds.push(Fd { n: 2 });
    fds.push(Fd { n: 3 });
    let mut it = fds.into_iter();
    let mut first = 0;
    while let Some(fd) = it.next() {
        first = close(fd);
        if first > 0 {
            break;
        }
    }
    let mut rest = 0;
    for fd in it {
        rest = rest + close(fd);
    }
    out.line(&first)?; // expect: 1
    out.line(&rest)    // expect: 5
}
```

The same goes for an empty list, however the checker knows it's empty: a
`while let Some(x) = l.pop()` that runs to its end, `List::new()`,
`is_empty()` in an `if`, or a function's result that says so. Generic code
too: `fn each<T, F: FnMut(T)>(l: List<T>, f: F)` can `for x in l { f(x) }`
for any `T`.

## What it's an impl of

`impl Discard for { v: T | p } { fn discard(self) { .. } }` is an impl of
`core::mem::DiscardWhen`, a trait of `core`'s that nothing can name, for
`T`, whose `discard`'s `self` is refined: `self: { v: Self | p }`, which a
program can't write. So everything an impl is checked for, it's checked
for (its trait's methods, the orphan rule, one impl per type), and its
`discard` is a function with a refined parameter, checked knowing `p`.
Only `Discard` can be implemented for a refined type: an impl of any
other trait, or an inherent impl, is of every value of its type.

It's never a bound. `List<Fd>` doesn't implement `Discard`, so `fn
sink<T: Discard>(x: T)` can't be given one, nor can its `discard` be
called by name: a drop is the only call of it. A type may have both: an
impl of `Discard` with bounds, as `impl<T: Discard> Discard for List<T>`,
and one for a refined type, as `List`'s for `l.len() == 0`. Where the
first applies, at a type the bounds hold of, it's what a drop calls.

## Why it's sound

> **Lemma W1 (a drop is a call).** Where the borrow checker finds a value
> of a type that isn't `Discard` dropped (a variable's scope ending, an
> assignment over it, a pattern not moving it out, docs/32.1), it was an
> error. Now, if the type has an impl of `DiscardWhen` (with the bounds in
> scope), the place is recorded instead (`Notes::drops_when`), and the
> refinement checker must prove `p` of its value there
> (`Walk::mir_drop_when`), as it would of the argument of a call of that
> impl's `discard`. Drop elaboration drops it with that `discard`. So the
> program runs as if the call were written there, and the call is
> checked as one: its `self` has its refinement.

> **Lemma W2 (every instance).** Monomorphisation's drop glue for `τσ`
> calls the `discard` of `τσ`'s impl of `Discard`, if it has one, which
> has no refinement; else that of its impl of `DiscardWhen`, which it has,
> as the one that applied at `τ` applies at `τσ` (an impl's header that
> matches a type matches its instances, lemma N2 of docs/73). Its `p` was
> proved of the value where it's dropped, and says only what measures
> give of that value, so it holds whatever `σ` is.

> **Lemma W3 (the impl's own body).** An impl of `DiscardWhen`'s
> `discard` is checked as an impl of `Discard`'s is: `self` is dropped
> part by part, and each part must be used up or `Discard` (lemma D3 of
> docs/20), knowing `p` of `self`, which every call of it proved. A
> `List`'s frees its buffer in one `unsafe` block, whose safety is that
> no slot holds a value: `len == 0`.

> **Lemma W4 (fail closed).** What the refinement checker doesn't know
> isn't proved. A value maybe moved, lent `&mut` to a call that says
> nothing of it, or a `for`'s iterator where a `break`, `return` or `?`
> leaves its loop: there it's known as no more than its type, and the
> drop is refused. A closure or a struct that owns an empty list is only
> `Discard` as its type says: its own fields aren't proved of, unless it
> has an impl of its own (as `Bag` above).

> **Lemma W5 (a `for` that runs to its end).** A `for` is `match
> into_iter(e) { mut it => loop { match next(&mut it) { Some(x) => body,
> None => break } } }`. The `None` arm's `break` is the one way out of the
> loop the body doesn't write. There, `next`'s result isn't `Some`, so by
> its refinement `old(it.len())` was `0`, and by what it ensures,
> `it.len()` is `0`: `it` may be dropped. Both are checked of `next`'s
> body, as any function's are.

> **Lemma W6 (what its `discard` uses).** A drop that may call an impl
> of `DiscardWhen`'s `discard` uses the value, as one that may call a
> `discard` of its own does (lemma D1 of docs/20): what it borrows must
> still be there, so the regions in its type hold to the drop
> (`moves::drop_uses`). And what it may write where its type is invariant
> is what lemma A9 of docs/68 says of a `discard` that isn't parametric.

The interpreter checks a refined parameter's refinement at each call, so
a drop the checker let through wrongly stops a run there; it also
reports a value that isn't `Discard` dropped by anything but its type's
impl of `DiscardWhen`. The adversarial tests are
`tests/t/err_soundness_discard_when_*.t`, and the generated family
`tests/generated_discard_when.rs`: a list of 0 to 3 values, made and used
up in one function in eleven ways (a `for` to its end, one that
`continue`s, `break`s or `return`s, `while let` and `pop`, one or two
`pop`s, nothing, its iterator dropped, its iterator partly by `while let`
and the rest by a `for`, `free_with` unless it's empty), of `Fd`s and of
`i64`s: each is accepted exactly when what's known says nothing's left,
and runs under the interpreter's oracle. Switching off W1 (the refinement
checker skipping the drop), W3 (the impl's `discard` not knowing `p`), W5
(`next` ensuring less), or making the glue call the refined impl where
the type is `Discard` (W2), each fails tests.

Writing the step found two ways it failed open, neither released. When
an impl of `DiscardWhen`'s `discard` had no refined `self`, the
refinement checker let its drops through; that can't be written, but
it's an internal error now, rather than a pass. And the borrow checker
didn't count a drop that calls such a `discard` as a use (W6), so a
value whose `discard` reads what it borrows could outlive that, which
the interpreter's oracle caught as a dangling reference
(`err_soundness_discard_when_borrow.t`).

## What it doesn't do

- **A bound.** `T: Discard` never holds by a refined impl. With refined
  type arguments (docs/60), `g::<{ l: List<Fd> | l.len() == 0 }>(l)`
  could, as every value of `T` in `g` would have the refinement.
- **`Option`.** `if let Some(fd) = o { close(fd) }` already drops a
  `None` of a linear type, as matching it by its variant uses it up
  (docs/06a.1). A `None` known some other way, `if o.is_none() {
  return }`, isn't dropped: `Option` has no impl for `o.is_none()`.
- **Closures.** A closure that owns an empty list is still linear: what
  it captured is checked by type.
- **Other collections.** Arrays of linear values can't be iterated by
  value at all (`IntoIter<T, N>` needs `T: Copy`), and a slice is
  borrowed.

## Decisions

- **D1: refined impls of `Discard`.** Not a second iteration protocol for
  linear iterators (`fn step(self) -> Option<(Item, Self)>`), which would
  fix `for` alone, with a second rewrite of it, and not empty lists or
  heaps.
- **D2: `List` and its `IntoIter`.** Not `Option` (patterns do it).
- **D3: drops only.** Never a bound; one per type, beside one with
  bounds, which a drop prefers.
- **D4: downstream now.** t-life's threads are joined by a `for`, and
  t-libs' heap has an impl of its own, so `into_sorted` drops the empty
  heap.

## Exercises

1. Write an impl of `Discard` for `{ o: Opt<Fd> | o.is_none() }` for an
   enum `Opt<T>` of your own, and drop a value of it after `if
   o.is_none() { .. }`. What does its `discard` do, and why can't it be
   empty?
2. `List`'s two impls of `Discard` agree on an empty `List<i64>`: which
   one runs? Why may they both exist, when two impls of one trait for a
   type can't?
3. Take the `ensures` off `IntoIter::next`. Which tests fail, and with
   what message? Which lemma did they need?
4. A `for` over `&mut l` never drops what's in `l`. Why is its iterator
   `Discard` with no refinement?
5. Why must `DiscardWhen` not be a bound? Write a generic function that
   would leak a value if `List<Fd>: Discard` held whenever its list is
   empty somewhere.

## Reading

- Wadler, *Linear types can change the world!* (1990): values that must
  be used exactly once, and the unrestricted ones that may be dropped.
- Walker, *Substructural Type Systems*, in Pierce (ed.), *Advanced Topics
  in Types and Programming Languages* (2005): weakening, the rule a
  `Discard` type has, and what a type without it means.
- Strom and Yemini, *Typestate: A Programming Language Concept for
  Enhancing Software Reliability* (1986): what may be done to a value
  depends on a state the checker follows, here a refinement.
- DeLine and Fähndrich, *Enforcing High-Level Protocols in Low-Level
  Software* (Vault, 2001): resources whose type says which state they're
  in, and may be freed only in some.
- Rondon, Kawaguchi and Jhala, *Liquid Types* (2008): refinements as
  subtypes, `{ v: T | p }` of `T`, of which an impl for the subtype is
  the one-trait case.
