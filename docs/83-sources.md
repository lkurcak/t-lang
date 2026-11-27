# Step 83: where values come from

Code: `src/check/refine/sources.rs` (`Signature::occurrences` and the
questions asked of it: `conserved`, `items_from_protocol`,
`closure_output`, `sole_source`, `kept`, `makes`); their callers in
`src/check/refine/items.rs` (`Walk::adapter_roles`, `Walk::item_params`,
`Walk::sealed_in_result`, `Walk::sealed`, `Walk::free_in_protocol`) and
`src/check/refine/targs.rs` (`Walk::instantiate_one`,
`Walk::closure_trees`); the pins in `src/borrowck/build.rs`
(`Builder::pin`) and `src/borrowck/loans.rs` (`pins`); and
`traits::normalise`.

Several of the checker's rules rest on parametricity: code generic in `X`
can't make a value of `X`, so every one it gives back is one it was given.
Step 60 knew what a call gives back of a refined type argument that way
(lemma A2 of docs/60), step 74 what a closure gives (C2 of docs/74), step
78 which closure that is (V5 of docs/78), step 63 what a closure written
in a call can count on of what it's given (W7 of docs/63), and step 82
what an iterator made by an adapter gives (I3, I4 of docs/82). Each rule
said, by a list of its own, which parts of a signature could give a value,
and refused those. Retrospective 13 found that the lists missed some: a
closure that's lent a `&mut X` can put another `X` there, and step 82's
lists looked only at closures' results. Writing this step found three
more. On 0.97.0, every one of these was accepted, and broke a type at run
time:

```rust,error
fn redo<X: Iterator, F: FnOnce(&mut X)>(x: X, f: F) -> X {
    let mut y = x;
    f(&mut y);
    y
}

fn main() {
    let a: [u8; 8] = [0; 8];
    // `redo` gives back the `100..101` the closure put where it lent it.
    for r in redo(0..8, |it: &mut core::ops::Range<usize>| *it = 100..101) {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

So this step turns the lists around. One analysis finds every place a type
parameter is in a signature, and each rule asks it, refusing any place it
doesn't know to be harmless: what nobody thought of is refused, not let
through.

## Every place a type parameter is

`Signature::occurrences` reads a function's scheme, or an impl, part by
part, each by name, so that a part added to either is a compile error
there until someone says what it gives. It finds each place a type
parameter `Z` is, its *site*, and how it's there, its *shape*:

| site | what it is to the code |
|---|---|
| a parameter's type | what it's given |
| the result's type | what it gives back |
| a field's type, of an impl's type | what its values hold |
| a bound, `T: Tr` | methods it can call, which may make one |
| an equality bound | what it says a type is |
| a closure's parameter, `F: FnMut(&mut Z)` | what it gives the closure, or what the closure can put back |
| a closure's result, `F: FnMut() -> Z` | what the closure gives |

| shape | |
|---|---|
| whole | `Z` |
| shared | `&Z` |
| read | further in, where it's covariant: `Option<Z>`, `(Z, u8)`, `&[Z]` |
| write | where another can be put: behind a `&mut`, in a `Cell` |
| hidden | in a function's type, an `impl` type, a projection's: not followed |

What's at a place is `Z` itself, its items (`Z::Item`), or another
projection of it. Each rule is then a question about those places:

- **`conserved`** (I3 of docs/82): an iterator the code can only pass on.
  Given (but not by a function value it's given: hidden); given back;
  bounded by `Iterator`, `DoubleEndedIterator`, `IntoIterator` and
  markers; its own associated types said; given to closures only to read.
  Not a closure's result, nor where a closure can write.
- **`items_from_protocol`** (I3′ below): `conserved`, and its items come
  only from its protocol: no equality says what they are, none is given
  or held, no closure gives one or can put one back.
- **`closure_output`** (I4 of docs/82): the one closure an adapter's `U`
  comes from: one closure's result is `U`, whole; nothing else gives one.
- **`sole_source`** (V5 of docs/78): the one parameter every value of a
  closure type comes from.
- **`kept`** (W7 of docs/63): a closure the callee may keep, to be called
  once it's returned: unless its only source is its parameter, and it's
  in neither the result nor a bound but its `Fn` traits' and markers',
  and the result has no `impl` type.
- **`makes`** (A2 of docs/60): a bound or closure that makes values the
  call can't follow. What a closure can write back through a parameter
  isn't one: a closure given a `&mut` to what's declared of a refined type
  argument must be walked keeping it (W7), which refuses one made before.

The holes, each refused now (`tests/generated/sources.rs` has them all,
with ways that must still be accepted):

- a closure that can write back where the callee looks: `&mut X`, `&mut
  Option<X>`, `&mut (X, u8)`, `&Cell<Option<X>>`, `&mut &mut X`; of an
  adapter's iterator, of its items, or of its closure's `U`;
- a second closure that gives an adapter's `U`, as a result (in an
  `Option` too) or fed back;
- a closure given to the callee that gives it the closure `map` is
  given;
- an equality bound saying what an iterator's items are, given to a
  closure walked as if they were the iterator's (H4, found writing this
  step):

```rust,error
fn need8(_r: { r: usize | r < 8 }) {}

// `X::Item = usize` lets `each` make an item of its own.
fn each<X: Iterator<Item = usize> + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {
    let _ = it;
    let mut f = f;
    f(100);
}

fn main() {
    each(0..8, |x: usize| need8(x));
}
// error: can't prove `r < 8` for argument `_r` of `need8`
```

## Every impl of the protocol

What an iterator gives comes from its impl of `Iterator`, and, if it has
one, of `DoubleEndedIterator`, whose `next_back` is a way to its items
too. Each has bounds of its own. Step 82 read only the first (H5 and H6,
found writing this step): an adapter whose `DoubleEndedIterator` impl
knows what its items are can make one, and so can an iterator that gives
what it holds:

```rust,error
#[derive(Discard)]
struct Nine<I> { iter: I }

impl<I: Iterator> Iterator for Nine<I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I: DoubleEndedIterator<Item = usize>> DoubleEndedIterator for Nine<I> {
    fn next_back(&mut self) -> Option<usize> with I::e {
        match self.iter.next_back() {
            Some(_x) => Some(9),
            None => None,
        }
    }
}

fn nine<I: Iterator>(iter: I) -> Nine<I> { Nine { iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in nine(0..8).rev() {
        let _ = a[r];
    }
}
// error: can't prove `i < self.len()` for argument `i` of `index`
```

So each value's place in what an adapter gives has a *role*
(`Walk::adapter_roles`): the items of an iterator it holds, or what a
closure it holds gives. The role is a place in the adapter's type, `I` in
`Nine<I>`; and in each impl of the protocol, the parameter at that place
must play it, its items from its protocol alone, or its closure the one
source of its result. Of an iterator that gives what it holds, the
parameter at the place must be free in each (`Walk::free_in_protocol`).
And the protocol's methods, which an impl may override, must only give
items to closures, never get one from them
(`protocol_methods_harmless`): that's a fact of `Iterator` as it is, which
a method added to it could change.

## Closures kept, and sealed adapters

A closure written in a call is walked knowing what the callee can give
it: the items of the iterators it's given, or the values of a refined type
argument (W7). That's what it's called with only while the callee runs. If
the callee keeps it, giving it back, or in an `Option`, an `impl` type, or
to another closure that does, it can be called with anything after:

```rust,error
fn need8(_r: { r: usize | r < 8 }) {}

fn keep<X: Iterator + Discard, F: FnMut(X::Item)>(it: X, f: F) -> F {
    let _ = it;
    f
}

fn main() {
    let f = keep(0..8, |x: usize| need8(x));
    f(100);
}
// error: can't prove `r < 8` for argument `_r` of `need8`
```

But `map` keeps its closure, in the `Map` it gives back, and walking that
closure knowing what the iterator gives is what makes `(0..4).rev().map(|x|
a[x])` check (docs/82). The difference is that only `Map`'s `next` can
call it. A struct whose fields its module alone sees is abstract
elsewhere (docs/06d): only its module's functions get at what it holds.
So an adapter is *sealed* where a call is (`Walk::sealed`) if:

- the code isn't in its module, nor in one inside it;
- its fields are private;
- no function of its module, nor of one inside it, takes one of it, or of
  a type that holds one, except the methods of its impls of the protocol
  and impls derived for markers and `Discard`: such a type is in no
  parameter, bound or equality, and in a result only the adapter itself,
  as plain data, which makes one; not a type holding one, nor a function
  or an `impl` type, which could take one (`-> impl FnMut(&mut S)`
  returns a way in);
- and in each impl of its protocol, the closure there is called with what
  the iterator at the callee's `X`'s place gives, whose items come from its
  protocol alone (`Walk::sealed_in_result`).

Then the closure kept in it is called only with items, by the protocol.
`core`'s `Map` and `Filter` are sealed; an adapter whose module gives its
closure back isn't:

```rust,error
fn need8(_r: { r: usize | r < 8 }) {}

#[derive(Discard)]
struct Each<I, F> { iter: I, f: F }

impl<I: Iterator, F: FnMut(I::Item) -> bool> Iterator for Each<I, F> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I, F> Each<I, F> {
    fn into_f(self) -> F where I: Discard { self.f }
}

fn each<I: Iterator, F: FnMut(I::Item) -> bool>(iter: I, f: F) -> Each<I, F>
where
    I::Item: Discard,
{ Each { iter, f } }

fn main() {
    let e = each(0..8, |x: usize| {
        need8(x);
        true
    });
    let f = e.into_f();
    let _ = f(100);
}
// error: can't prove `r < 8` for argument `_r` of `need8`
```

An adapter of the program's own, in a module of its own, is sealed as
`Map` is:

```rust
mod each {
    #[derive(Discard)]
    pub struct Each<I, F> { iter: I, f: F }

    impl<I: Iterator, F: FnMut(I::Item) -> u8> Iterator for Each<I, F> {
        type Item = u8;
        effect e = I::e;

        fn next(&mut self) -> Option<u8> with I::e {
            match self.iter.next() {
                Some(x) => Some((self.f)(x)),
                None => None,
            }
        }
    }

    pub fn each<I: Iterator, F: FnMut(I::Item) -> u8>(iter: I, f: F) -> Each<I, F> {
        Each { iter, f }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 4] = [7, 8, 9, 10];
    for v in each::each(1..3, |x: usize| a[x]) {
        out.line(&v)?;
    }
    // expect: 8
    // expect: 9
    Ok(())
}
```

## Finding a place, before or after

A call takes its borrows of places after its other arguments (docs/06),
and since 0.96.0 so does the reborrow of a `&mut` argument (lemma A4 of
docs/81). Docs/06 said that finding the place, then, is following fields
and `*`s, which nothing can see happen. Retrospective 13 found that's not
so: a later argument can give a reference on the way a new value, and then
which place is borrowed depends on when it's found. 0.95.0 found it first,
and wrote `a`; 0.96.0 found it last, and wrote `b`; Rust finds it first.
Now it's refused, so the place is the same whenever it's found:

```rust,error
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) {
    g.x = v;
}

fn f(a: &mut Game, b: &mut Game) {
    let mut g = a;
    put(g, {
        g = b;
        1
    });
}

fn main() {
    let mut a = Game { x: 0 };
    let mut b = Game { x: 0 };
    f(&mut a, &mut b);
}
// error: can't give `g` a new value here: an earlier argument borrows `*g`
```

Each reference or box a waiting borrow goes through is *pinned*
(`Builder::pin`): read where the argument is, so a `&mut` to it in use
there clashes (another way to change it), and from there until the
borrows are taken, no later argument may give it a new value, lend it
`&mut`, or move it, nor a place it's in short of a `*` (`loans::pins`).
What's behind it is still theirs to change, which is what args first is
for:

```rust
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) {
    g.x = g.x * 10 + v;
}

fn at(g: &mut Game) -> u32 { g.x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut game = Game { x: 0 };
    let g = &mut game;
    put(g, {
        g.x = 3;
        1
    });
    put(g, at(g));
    out.line(&game.x)?; // expect: 341
    Ok(())
}
```

## A closure's parameter through an equality

Writing the tests for H4 found that calling a closure whose parameter is
`X::Item`, where an equality bound says that's `usize`, stopped the
borrow checker: the closure's signature, `F::SIG = fn(X::Item)`, was what
the bound said, but not worked out further with the other bound
(`traits::normalise`). Now what a bound says a projection is, is
normalised with the others, each used once on the way, so bounds that say
it of each other end.

## Why it's sound

- **P1 (every place).** A generic body's typing uses only its context:
  its parameters, the bounds in scope and the impls they make available,
  the equality bounds, and, of a type it can see into, its fields. A value
  of `Z` is made by none of the language's own forms (Reynolds), so it
  comes from one of those: a parameter, a field, a bound's method, a type
  an equality says it is, or a closure, through its result or through a
  parameter it can write. `occurrences` finds `Z` at every one of them;
  the parts of a scheme and an impl are each read by name, and every kind
  of type is matched, so nothing is skipped by default.
- **P2 (refused unless known).** Each question accepts only sites and
  shapes it has a reason for, and refuses the rest, so a way in nobody
  thought of isn't taken as harmless. Where a shape is covariant
  ("read"), what has a value there can read it, not put one back, as
  subtyping goes only one way there.
- **I3′ (items from the protocol).** If `items_from_protocol(X)`, every
  value of `X::Item` the body has came from a call of `X`'s protocol on
  an `X` it has (by P1, no other site gives one), and those are the ones
  it was given, or what they became (I3). So what's given to a closure as
  an `X::Item` is one of what the given iterators give.
- **I4′ (every impl).** The values an adapter's protocol gives come from
  its `Iterator` and `DoubleEndedIterator` impls' methods, each checked
  against its own bounds. If at each role's place every impl's parameter
  is an iterator whose items come from its protocol, or a held closure
  that's its result type's one source, then by I3′ and C1/C2 of docs/74
  each value is one of those (I4, now of each impl).
- **S (sealed).** Outside its module, a struct with private fields is an
  existential type (Mitchell and Plotkin): only its module's code gets at
  what it holds, its functions and the closures they make. A value of it
  gets to that code only through a function's parameter (P1, with the
  struct as the type), or a closure or function value of the module's,
  which the code outside gets only from a result that has a function or
  `impl` type, or a type holding one; so only the protocol's methods, and
  the derived impls of markers and `Discard`, which call no closure, get
  one. Those,
  by I4′, call the closure only with what the iterator at the callee's
  `X`'s place gives, which is what the walk gave the closure.
- **A5 (pins).** If nothing on the way to a place, its references and
  boxes, changes between where the argument is and where the borrow is
  taken, following the way gives the same place at both. Changing one
  needs a write to it or a place it's in (refused), or through another
  reference to it, which is a loan of it in use where the argument is
  (the read there clashes), or made after it, by a `&mut` of it (refused).

The interpreter checks every refinement a value is given to, so a value
that came from where a rule didn't look would stop a test there.

## What it doesn't do

- **Lifetimes.** Whether a `&mut` parameter is call-scoped or lend-framed
  (docs/30, docs/59) is a question of where its lifetime is, which those
  rules answer of their own; what they let the caller count on is what the
  callee proved after any lend.
- **Read shapes given back.** A parameter of type `Option<X>` gives the
  callee an `X` it was given, which `conserved` accepts; but what it gives
  back isn't followed into the `Option` (`given_items`), so its items are
  forgotten.
- **Adapters that can't be sealed.** A closure kept in an adapter whose
  module has other ways to it isn't walked knowing what it's given, even
  if those ways don't call it.

## Decisions

From retrospective 13 (docs, `t-retro13-findings`):

- **D1 one analysis**, which every rule resting on parametricity asks
  (recommended, chosen), rather than fixing each list.
- **D2 sealed adapters**: a closure kept in the callee's result is walked
  knowing what it's given only if the adapter is sealed (recommended,
  chosen); `core`'s `Map` and `Filter` are.
- **D3 refuse the change**: a later argument may not give a reference a
  waiting borrow goes through a new value (recommended, chosen), rather
  than finding the place first, as Rust does, which would make a
  `put(g, at(g))` that reads through `g` clash.

## Exercises

1. Which shape is `X` in `&mut Option<&X>`, and why does `conserved`
   refuse it as a closure's parameter?
2. Give `Filter` an inherent method `fn predicate(&self) -> &P`. Which
   test now fails, and why must it?
3. Write a function generic in `T` with a parameter `g: fn() -> T`. Why
   is that occurrence *hidden*, and what would go wrong if it counted as
   given?
4. A pin is read where its argument is. Write a program a pin with no
   read would accept, where the place changes anyway.
5. Why may `items_from_protocol` accept `X: IntoIterator<IntoIter = Y>`,
   for another type parameter `Y`?

## Reading

- Reynolds, *Types, Abstraction and Parametric Polymorphism* (1983): what
  generic code can do with what it's given.
- Wadler, *Theorems for free!* (1989): a function's type says what it
  gives back.
- Mitchell and Plotkin, *Abstract Types Have Existential Type* (1988):
  sealed adapters.
- RFC 2025, *two-phase borrows*, and RFC 2094, *non-lexical lifetimes*
  (its "fake borrows" of a `match` guard): Rust's ways to keep a place
  from changing before it's used, which pins are.
