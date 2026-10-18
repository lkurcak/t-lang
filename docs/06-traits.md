# Step 6a: traits

Code: `src/traits.rs` (traits, impls and resolution), `src/check/traits.rs`
(collecting impls, coherence, termination, method lookup), `src/check.rs`
(T-Method, T-Path, T-Eq), `src/borrowck/build.rs` (a call's borrows, `call_into`) and
`src/mono.rs` (calls become impls' methods).

```rust
trait Show {
    fn show(&self);

    fn show_twice(&self) {
        self.show();
        self.show();
    }
}

#[derive(Discard)]
struct Point { x: int, y: int }

impl Show for Point {
    fn show(&self) {
        print(self.x);
        print(self.y);
    }
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

impl<T: Show> Show for Option<T> {
    fn show(&self) {
        match self {
            Option::Some(x) => x.show(),
            Option::None => print(0),
        }
    }
}

fn show_all<T: Show>(a: &T, b: &T) {
    a.show();
    b.show();
}

fn main() {
    let p = Point { x: 1, y: 2 };
    p.show_twice(); // expect: 1
                    // expect: 2
                    // expect: 1
                    // expect: 2
    show_all(&Option::Some(Point { x: 3, y: 4 }), &Option::None);
    // expect: 3
    // expect: 4
    // expect: 0
}
```

Step 4's generics are *parametric*: `fn id<T>(x: T) -> T` does the same
thing whatever `T` is, and can't do anything with a `T` but move it
around. Traits add *ad-hoc* polymorphism: a function that does something
different for each type, defined separately for each, in an *impl*. A
*bound* `T: Show` lets generic code use it.

## Traits are predicates on types

Read `trait Show { .. }` as declaring a predicate, `Show(T)`, true of the
types that have a `show`. An impl is a rule for proving it:

```text
impl Show for Point                 ⊢ Show(Point)
impl<T: Show> Show for Option<T>    Show(T) ⊢ Show(Option<T>)
```

So `Show(Option<Option<Point>>)` holds, by the second rule twice and then
the first. That's a Horn clause, and finding the proof is Prolog's search,
*resolution*, with types for terms: find the impl whose type matches, then
prove what it needs (`Traits::implements` in `src/traits.rs`). This is
Haskell's type classes (Wadler and Blott, 1989), and Rust's traits.

A bound is an assumption. In the body of `fn show_all<T: Show>`, `T: Show`
holds, though nothing says which impl it's by; that's for each caller to
show, for the `T` it picks. Each obligation, such as `int: Show` for
`show_all(&1, &2)`, waits until its type is known, at the end of the
function, as `==` on a type to be inferred did in step 3.

```rust,error
trait Show {
    fn show(&self);
}

fn show_all<T: Show>(a: &T, b: &T) {
    a.show();
    b.show();
}

fn main() {
    show_all(&1, &2);
}
// error: `show_all` needs its `T` to implement `Show`, and `int` doesn't
```

A *supertrait* is a rule the other way: `trait Ord: Eq` says every `Ord`
type is `Eq`, so implementing `Ord` needs `Eq` implemented, and a bound `T:
Ord` gives `T` the methods of both.

### Two rules that keep the search simple

**Coherence.** No two impls of a trait can be for the same type, whatever
their type parameters are. Then there's never a choice of impl to make,
and what a call runs doesn't depend on which impls are in sight. Overlap is
*unification* of the impls' types, with their parameters for variables
(`traits::overlap`), and bounds don't count:

```rust,error
trait Show {
    fn show(&self);
}

trait Print {
    fn print(&self);
}

impl<T: Show> Print for T {
    fn print(&self) { self.show() }
}

impl Print for int {
    fn print(&self) { print(*self) }
}
// error: this impl of `Print` overlaps `impl Print for T`
```

`int` isn't `Show`, so the two can't both apply today. But if someone
made it `Show`, the impl for `int` would quietly stop being the only one:
an impl far away would change what code here means. Rust decided the same,
and has *specialization* as an unfinished way out; T has none.

**Termination.** Proving a goal by an impl needs what the impl needs. Each
impl's type parameters are parts of its type (an impl whose parameter
isn't, like `impl<T> Show for int`, is rejected: nothing could say what
`T` is). So what an impl needs is about smaller types than what it proves,
and the search ends: the *Paterson conditions* (Sulzmann et al., 2007),
which a bound on a type parameter meets whenever the impl's type is more
than that parameter.

A *blanket impl*, for a bare parameter, needs the same type again:

```rust
trait Show {
    fn show(&self);
}

trait Describe {
    fn describe(&self);
}

impl<T: Show> Describe for T {
    fn describe(&self) {
        print(0);
        self.show();
    }
}

impl Show for int {
    fn show(&self) { print(*self) }
}

fn main() {
    5.describe(); // expect: 0
                  // expect: 5
}
```

Proving `int: Describe` needs `int: Show`: the same size, but another
trait. That ends unless there's a way back, so the blanket impls between
traits can't form a cycle. Strict Paterson would reject this one too;
T only rejects what could actually loop:

```rust,error
trait A {}
trait B {}
impl<T: A> B for T {}
impl<T: B> A for T {}
// error: proving that a type implements `B` could go on forever
```

## Methods

`x.m(a)` is a call of `m` with `x` as its first argument, its *receiver*.
Which `m` depends on what `x` is, so its type must be known there, as for
a field. The rule is deliberately simple:

```text
  Γ ⊢ e : &..&S    m is a method of S of type fn(R, T1, .., Tn) -> U
  R is S, &S or &mut S    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
 ------------------------------------------------------------------ (T-Method)
                  Γ ⊢ e.m(a1, .., an) : U
```

Follow references and boxes from `e` to `S`, as a field does, then take
`&` or `&mut` of that if the method wants it. At most one `&` is inserted,
and lookup is by `S` alone. An *inherent* method, from an `impl S`, comes
first; then a trait's, of a trait some impl could be for `S` (or, for a
type parameter, one its bounds say). Two traits with a method of that name
are ambiguous; the path `Trait::m(x)` says which.

Rust's rule is a search: it tries `S`, `&S`, `&mut S`, then `*S` and so on,
inherent before trait at each step. That finds `impl Show for &Point`'s
methods from a `&Point`, which T's rule doesn't, as it looks through the
`&` to `Point`. Such an impl's methods are only reached through a type
parameter, as in `show_all` at `T = &Point`. Nothing has needed more yet,
and the simple rule never has to choose between two steps of a search.

A method without `self` is only reached by a path: `Point::new(1, 2)`. A
path without `(..)` is the method as a function, `let f = Point::len;`.
`Self` is the impl's type, in it.

### A call's borrows come last

```rust
#[derive(Discard)]
struct Stack { top: int, size: int }

impl Stack {
    fn len(&self) -> int { self.size }

    fn push(&mut self, x: int) {
        self.top = x;
        self.size += 1;
    }
}

fn main() {
    let mut v = Stack { top: 0, size: 0 };
    v.push(v.len());
    print(v.size); // expect: 1
}
```

`v.push(v.len())` is `<Stack>::push(&mut v, <Stack>::len(&v))`, and the
`&mut v` comes first. Taken literally, `&v` can't happen while it lasts.
So a call works out its arguments in order but takes its borrows of
places, `&v` and `&mut v`, written or inserted for a receiver, after the
rest, just before the call: `v.push(v.len())` is `let n = v.len();
v.push(n)`. The arguments can do anything to `v`, even change it, but a
borrow they make can't still be in use at the call, as the `&mut v` is
taken there.

Only the borrow waits: what a place is made from, if it isn't a variable,
like `make()` in `&mut make().f`, is worked out in order. Finding a place
is only following fields and `*`s of references and boxes, which nothing
can see happen, so taking it later changes no program's meaning; it only
lets the borrow checker see that nothing clashes. That's a reason to keep
it so: when traits for `*` and indexing come, their calls will be worked
out in order, and only what's built in borrowed late.

Rust has *two-phase* borrows instead (RFC 2025): the receiver's `&mut` is
taken first but only *reserved*, working as a shared borrow until the
call. That's needed there because finding a place can run code, through
`Deref` and `Index`, so it can't wait. It accepts less: `v.push({ v.clear();
1 })`, and `f(&mut v, v.len())` with the `&mut` written, are fine in T.

## `Eq` and `Copy`

`==` is `Eq`'s: `l == r` is `Eq::eq(&l, &r)`, and `!=` its negation. That
replaces step 1's rule that `==` works on anything without a function in
it, and step 4's error for `==` on a `T` now has its answer, `T: Eq`.
`int`, `bool` and `()` are `Eq`, and references and boxes if what they
point to is. A struct or enum is if it says so, with `#[derive(Eq)]` to
compare field by field (each field by its own type's `==`, so a field with
an `eq` of its own counts it), or with an impl. Functions never are.
`Eq` has only `eq`, so `==` and `!=` can't disagree.

`Copy` is a trait with no methods: a *marker*. `#[derive(Copy)]` is an
impl, and so is `impl Copy for P {}`, which also needs every field `Copy`.
A bound `T: Copy` lets generic code copy a `T`. Since step 6a.1, `Copy`
needs `Discard` too (docs/06a.1-linear-types.md).

A derived impl needs of the type's parameters only what its fields need:

```rust
#[derive(Discard, Copy)]
struct Callback<T> { f: fn(T) -> int }

fn unbox(b: Box<int>) -> int { *b }

fn main() {
    let c = Callback { f: unbox };
    let d = c;
    print((c.f)(Box::new(1)) + (d.f)(Box::new(2))); // expect: 3
}
```

`Callback<Box<int>>` is `Copy`, as a function is. Rust's derive would
need `T: Copy`: it bounds every parameter, whatever the fields need, which
is why the trivial impl sometimes has to be written by hand. Rust keeps
that because bounds worked out from fields make a type's public bounds
depend on its private fields, so changing a field could break users. T has
no libraries yet, so that can't happen; it will need thought when it does.
The bounds are a least fixed point, as a type can hold itself through a
reference (`check/traits.rs`, `derive`).

## Monomorphisation finds the impl

Each copy monomorphisation makes knows its types, so a call of
`<Show>::show` at `Self = Point` becomes a call of `<Point as Show>::show`,
the impl's method, or the trait's default if the impl leaves it out.
Nothing at run time looks anything up: this is *static dispatch*, and
after it there are no traits, only functions (`t mono` shows them). The
borrow checker runs before, on generic code, where it only needs the
method's signature, which is the same in every impl.

A trait call can hide polymorphic recursion (docs/04-generics.md):

```rust,error
trait Tr {
    fn f(&self) -> int;
}

#[derive(Discard)]
struct W<X> { x: X }

impl<X: Tr> Tr for W<X> {
    fn f(&self) -> int { (W { x: self }).f() }
}

impl<'a, X: Tr> Tr for &'a X {
    fn f(&self) -> int { (**self).f() }
}
// error: polymorphic recursion
```

So the check follows each trait call to every impl it could run. Step 4's
graph had edges that grow a type or keep it; an impl's parameters are
*parts* of `Self`, so those edges shrink it, by how deep they are. The
types grow without end exactly when some cycle adds more than it takes
away: a positive cycle, found by Bellman–Ford (`generics.rs`). That lets
`W<T>` show an `Option<&T>`, which shows a `&T`, which shows a `T`.

## Design decisions worth noticing

- **Static dispatch only.** No `dyn Trait`, so no vtables, which is no
  runtime. It'll come, as `&dyn Trait`, when a program needs one.
- **Coherence is global**, one impl per type in the whole program. There
  are no modules yet, so no orphan rules; they come with modules.
- **No implied bounds.** `struct Sorted<T: Ord>` makes every use show
  `T: Ord`, impls included. Rust's implied bounds for lifetimes are where
  rust#25860 comes from.
- **A trait impl holds at every lifetime.** Each lifetime and type
  parameter in its type is written once, with no lifetime bounds:
  `impl<'a> Tr for Pair<'a, 'a>` is rejected, as resolution ignores
  lifetimes and would find it for any `Pair`. (Haskell 98 asks the same of
  instance heads.) Rust instead has resolution emit lifetime constraints.
- **An impl's methods have exactly the trait's signatures**, up to the
  names of their own type parameters and lifetimes.
- **Obligations never drive inference.** "There's only one impl, so it must
  be that" would make adding an impl break code.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| associated types, `type Item;` | the next part: docs/06b-associated-types.md | 6b |
| closures as `Fn` traits, capturing by reference | after that | 6c |
| `dyn Trait` | static dispatch until a program needs it | — |
| `Drop` | drop check is the hard part | — |
| `for` loops | an `Iterator` trait, with associated types (6b) and closures (6c) | — |
| operator traits but `Eq` (`Add`, `Ord` for `<`) | not needed yet | — |
| traits with type parameters, `where` clauses | not needed yet | — |
| `impl Tr for &U` methods by `.` | the one-`&` rule | — |

## Exercises

1. Write two impls that overlap only if some type implements a trait, and
   see them rejected. Then argue that accepting them would be unsound in a
   program split into libraries, but sound in one file. What would T need
   to accept them safely?
2. `impl<T: A> B for T` with `impl<T: B> A for T` is rejected. Give a type
   that would make resolution loop, and trace it.
3. Change method lookup to Rust's autoref search, so `impl Show for &Point`
   is found from a `&Point`. What new ambiguity does that bring?
4. Make `call_into` take each borrow where its argument is, and see which
   tests fail. Then find a program that would go wrong if a borrow were
   taken late but its place worked out early, in the interpreter too.
5. A derived `Copy` takes its bounds from the fields. Write a type whose
   least fixed point needs more than one round.
6. Write a program whose polymorphic recursion check fails though
   monomorphisation would end. (Hint: the weights are bounds on depth, not
   depths.)
7. Dictionary passing: Haskell compiles `Show a => a -> ..` to a function
   that takes the impl's methods as an extra argument. Sketch what T's
   `show_all` would compile to, and what it would cost compared with
   monomorphisation.

## Reading

- Philip Wadler and Stephen Blott, *How to make ad-hoc polymorphism less
  ad hoc*, POPL 1989. Type classes.
- Mark P. Jones, *Qualified Types: Theory and Practice*, 1994. Predicates
  in types, which bounds are.
- Martin Sulzmann, Gregory Duck, Simon Peyton Jones and Peter Stuckey,
  *Understanding Functional Dependencies via Constraint Handling Rules*,
  JFP 2007. Where the Paterson conditions are stated.
- Nicholas Matsakis, *Little Orphan Impls*, 2015, and RFC 1023: *Rebalancing
  coherence*. Coherence across crates.
- RFC 2025: *Enable use of two-phase borrows*. Rust's answer to
  `v.push(v.len())`.
- RFC 1210: *impl specialization*, and why it's still unstable.
