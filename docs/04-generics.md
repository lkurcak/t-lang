# Step 4: generics

Code: `src/types.rs` (`Type::Param`, `Subst`, `Scheme`), `src/check.rs`
(`Checker::lookup` and `Checker::instantiate`), `src/check/generics.rs`, and
`src/mono.rs`.

```rust
enum Option<T> { Some(T), None }

fn map<T, U>(o: Option<T>, f: fn(T) -> U) -> Option<U> {
    match o {
        Option::Some(x) => Option::Some(f(x)),
        Option::None => Option::None,
    }
}

fn main() {
    let n = 10;
    match map(Option::Some(4), |x| x * n) {
        Option::Some(y) => print(y), // expect: 40
        Option::None => {},
    }
}
```

`map` works for every `T` and `U`. Nothing at the call says which: the
checker works out `T = int` and `U = int` from the arguments. Then, before
the program runs, `map` is copied into a version just for `int`s, and that's
what runs. This step is about both halves: what a type like "for every `T`"
means, and how to compile it with no runtime.

## Parametric polymorphism

Until now every function had one type. `fn id(x: int) -> int { x }` works for
ints only, and the same function for bools has to be written again. A
*generic* function is written once, for every type:

```rust
fn id<T>(x: T) -> T { x }
```

Its type is `∀T. fn(T) -> T`, "for all types `T`, a function from `T` to
`T`". In the code that's a `Scheme`: a list of type parameters and a type
that mentions them (`Type::Param`).

This is *parametric* polymorphism: `id` does the same thing whatever `T` is,
because it can't find out what `T` is. That's the other kind of polymorphism
from overloading (*ad-hoc* polymorphism, step 6), where `+` on ints and `+`
on floats are different code chosen by type.

Structs and enums take type parameters too, and that's where generics earn
their keep: one `Option<T>`, not an `OptionInt`, `OptionBool` and
`OptionShape`. `Option` on its own isn't a type; it's a *type constructor*,
a function from types to types, and `Option<int>` is a type. Reading a field
of an `Option<int>` substitutes `int` for `T` in the declared field types:
`types::struct_fields` and friends do that, so nothing else in the checker
has to.

## Checking: once, with `T` opaque

There are two ways to check a generic function.

C++ templates check each *use*: `id<int>` is checked as if written out with
`int`, then `id<bool>` with `bool`. An error in a template shows up at the
use that triggers it, deep in someone else's code, and a template's
signature promises nothing.

T, like Rust, ML and Haskell, checks the *definition*, once. Inside `id`, `T`
is a type the checker knows nothing about. `Type::Param("T")` unifies with
itself and nothing else: not `int`, not `bool`, not `U`
(`err_generic_opaque.t`, `err_generic_param_distinct.t`):

```rust,error
fn add_one<T>(x: T) -> int { x + 1 }
// error: expected `int`, found `T`
```

If the body checks with `T` opaque, it checks for every `T`, and callers only
need the signature. This is what *parametricity* is about: since the body
can't look at a `T`, `fn(T) -> T` can only return its argument (or never
return). Philip Wadler called this "theorems for free": the type alone
says what the function does.

### `==` needs to know

T-Eq (step 1) bans `==` on functions, so it can't allow `==` on a `T`
either: the caller could pick a function type.

```rust,error
fn same<T>(a: T, b: T) -> bool { a == b }
// error: cannot compare values of type `T` with `==`: `T` is a type
// parameter, so it may be a function; ...
```

Saying "any `T` that has `==`" takes a *bound*, `T: Eq`. That's a trait, and
traits are step 6. The same goes for `Option<T>`, which contains a `T`.

## Using a generic: instantiation

At each use, a generic's type parameters are *instantiated*: each gets a
fresh type variable, exactly like the ones step 3 made for an unannotated
closure parameter. Then inference carries on as before:

```text
id(1)        id : fn(α) -> α       (α fresh, for T)
             1 : int = α           α = int
             id(1) : int
id(true)     id : fn(β) -> β       (β fresh: a new use, a new variable)
             ...                   β = bool
```

That's `Checker::lookup` for functions and `Checker::instantiate` for struct
literals, variants and patterns. As with every other variable, `solve_all`
demands that each one is solved by the end of the function. `let x =
Option::None;` doesn't say what `T` is, so it's an error asking for an
annotation (`err_generic_infer.t`).

Or the type arguments can be written out, with Rust's *turbofish*:
`id::<int>(1)`, `Option::<int>::None`, `Pair::<int, bool> { .. }`, and the
same in patterns (`generic_turbofish.t`). The `::` is there because
`id<int>(1)` would read as `id < int > (1)`, two comparisons. Written type
arguments simply take the place of the fresh variables, so everything else
works the same. They're what makes a type parameter that the signature
doesn't mention usable: `fn none<T>() -> Option<T>` can be inferred from
where its result goes, but `fn count<T>(n: int) -> int` can only be called
as `count::<bool>(3)` (`err_generic_infer_fn.t`). A struct can have such a
*phantom* parameter too: `struct Id<T> { n: int }` is just an int, but an
`Id<User>` and an `Id<Order>` can't be mixed up.

Inference stays inside function bodies. Generic signatures are written by
hand, and a body's variables never leak out of it.

## System F, Hindley–Milner, and what T does

Where can a `∀` go? That one question separates the classic systems.

**System F** (Girard 1972, Reynolds 1974) lets `∀` go anywhere a type can.
A function can take a polymorphic function as an argument:

```text
apply_both : (∀T. fn(T) -> T) -> (int, bool)
apply_both(f) = (f(1), f(true))
```

`f` is used at two types inside `apply_both`, so `f` itself must be
polymorphic: a `∀` *inside* an argument type. That type has *rank 2*: the
rank is how deeply a `∀` is nested to the left of arrows. System F has every
rank. It's very expressive, and type inference for it is undecidable (Wells,
1994): programs have to say where every `∀` goes and what each one is
instantiated with.

**Hindley–Milner** (Hindley 1969, Milner 1978, Damas and Milner 1982)
restricts `∀` to the outside of a type: `∀T. fn(T) -> T` but never
`fn(∀T. ...) -> ...`. That's *rank 1*, or *prenex* polymorphism (all the
quantifiers out front, in a prefix). Types with `∀` are called *type schemes*
and only `let`-bound names have them. In return, inference is complete:
every program has a most general type, found by unification, with no
annotations at all. And `let` *generalises*: `let id = |x| x` gets the scheme
`∀T. fn(T) -> T`, and each use of `id` gets its own copy. That's ML,
OCaml, and the core of Haskell.

**T** is rank 1 too, but it doesn't generalise:

| | where `∀` can go | where it comes from | inference |
|---|---|---|---|
| System F | anywhere | written | none in general (undecidable) |
| Hindley–Milner | outermost, on `let`-bound names | inferred, by generalising at `let` | complete, no annotations |
| T (and Rust) | outermost, on `fn`s, structs and enums | written: `<T>` | inside bodies only |

So only functions (top-level or nested), structs and enums can be generic,
and only by saying so with `<T>`. A closure, and any `let`, has one type
(`err_infer_monomorphic.t`, `err_generic_let_mono.t`):

```rust,error
fn id<T>(x: T) -> T { x }

fn main() {
    let f = id;          // id instantiated once: f : fn(α) -> α
    print(f(1));         // α = int
    print_bool(f(true)); // error: expected `int`, found `bool`
}
```

Why not generalise `let`, when HM shows how? It would make closures generic,
and a generic closure can't be monomorphised the way the next section does
it: a closure is a *value*, made at run time, holding its environment, so
copying it per type means copying values, not code. Rust makes the same
choice. And without generalisation, step 3's local inference is untouched:
type variables are made and solved inside one body, schemes are only ever
written by hand.

## Monomorphisation: why rank 1 matters

T has to compile to bare metal (see the README): every value needs a size
known at compile time, and there's no runtime to carry type information. But
a `T` could be an `int`, or a `Pair<int, int>` twice the size. How big is
the `x` in `fn id<T>(x: T) -> T`?

There are two classic answers.

- **Boxing** (Java, OCaml, Haskell): every value of unknown type is a
  pointer, so every `T` is one word, and `id` is compiled once. That needs a
  heap and something to free it, which T doesn't have.
- **Monomorphisation** (C++, Rust, MLton): compile a separate copy of `id`
  for each type it's used at. `id::<int>` moves 8 bytes, `id::<Pair<int,
  int>>` moves 16, and each is ordinary non-generic code.

T monomorphises, and `src/mono.rs` does it as a real pass: it takes the
checked program and makes a new one with no generics in it, which the
interpreter then runs. `t mono` prints it:

```sh
$ t mono tests/t/generics.t
...
fn map::<int, int>(o: Option::<int>, f: fn(int) -> int) -> Option::<int> {
    match o {
        Option::<int>::Some(x) => Option::<int>::Some(f(x)),
        Option::<int>::None => Option::<int>::None,
    }
}
```

Starting from every non-generic function, it reads the type arguments the
checker worked out for each use of something generic (`Info::instances`),
and asks for that copy. Inside a copy of a generic function, those type
arguments can mention its own type parameters, so the copy for `T = int`
substitutes `int` into them first. Each copy may need more copies, and so on
until nothing new is needed. `tests/t/*.mono` holds what it makes for a few
programs, and the golden tests check it.

This only works because of rank 1. Monomorphisation needs to know, at
compile time, every type each generic is used at. With rank 1 that's always
so: every `∀` is at the outside of an item (a `fn`, struct or enum), and
every use of one is a direct mention of that item's name, so each use has
its type arguments right there, and they're built from types known at the
use. Follow those
from `main` and you have them all.

With rank 2, `apply_both(f)` uses `f` at `int` and `bool`, but `f` is a
parameter: which function is that? Any polymorphic function a caller passes
in, at run time. There's no name to copy, so no copy to make: `f` would have
to be compiled once for every type, which is boxing again (or passing type
information at run time, as in Swift). That's why a language that
monomorphises keeps polymorphism prenex. In Rust it's the same, and it's why
a `dyn Trait` method can't be generic.

### Polymorphic recursion

Rank 1 is necessary but not quite enough. This is rank 1, and checks fine:

```rust
fn nest<T>(x: T, n: int) -> int {
    if n == 0 { 0 } else { nest(Option::Some(x), n - 1) }
}
```

But `nest::<int>` calls `nest::<Option<int>>`, which calls
`nest::<Option<Option<int>>>`, and monomorphisation never ends. A function
calling itself at a different type is *polymorphic recursion* (Mycroft,
1984). HM can't even infer it, since inside its own definition a function
isn't generalised yet. With signatures it can be checked, and boxing
languages like Haskell allow it, but it can't be monomorphised.

So the checker rejects it (`generics::polymorphic_recursion`,
`err_generic_poly_rec.t`). The precise rule: draw a graph with a node for each
type parameter of each item. A use of `g<.., A, ..>` inside `f` gives an edge
from each parameter of `f` that `A` mentions to that parameter of `g`, and
the edge *grows* if `A` is anything more than that parameter itself. Every
copy's type arguments are built along these edges, so monomorphisation goes
on forever exactly when some cycle has an edge that grows. That catches
mutual recursion (`err_generic_poly_rec_mutual.t`) and types too:
`struct Tree<T> { deeper: fn(int) -> Tree<Pair<T>> }` needs a `Tree<Pair<T>>`
for its field's type, then a `Tree<Pair<Pair<T>>>`
(`err_generic_poly_rec_type.t`). Recursion that changes type arguments
without growing them, like a `count<T>` that calls `count<int>` or a
`flip<A, B>` that calls `flip<B, A>`, needs only finitely many copies, so
it's allowed (`generic_recursion_ok.t`).

### What checking once can't see

There's one rule that checking a generic body once doesn't cover: closures
that capture are downward-only (step 3). Inside

```rust
fn id<T>(x: T) -> T { x }
```

returning `x` is fine for any `T` the checker can see, since `T` isn't a
function type. But its copy for `T = fn(int) -> int` returns a function
parameter, which may be a closure that captures, and that's exactly what
step 3 forbids. Whether a value may hold a stack frame isn't part of its
type, so a body checked with `T` opaque can't know. That rule isn't
parametric.

So monomorphisation's output is checked again (`mono::monomorphise`), and an
error in a copy is reported at the use that made it:

```
type error: cannot use `id` with `T` = `fn(int) -> int` here: in that copy of
it, cannot return `x`: it may be a closure that captures, and closures that
capture can only be passed down, not returned
```

That includes `id(inc)` for a top-level `inc`, just as step 3 rejects a
non-generic `fn id(f: fn(int) -> int) -> fn(int) -> int { f }`. This is a
small taste of C++'s errors-at-instantiation, kept to one rule. It goes away
when the type says where a value may live: lifetimes, step 5, where a `T`
that holds a borrow is written `T: 'a` and checked like any other bound.

## Leaving room for effects

Step 7 adds effects to function types: whether a call may panic, allocate or
diverge. Two things here are shaped for that:

- A function type is a `FnType` struct (`Type::Fn(FnType)`), not a tuple of
  parameters and result, so it can get an effects field. Code that walks
  types goes through `Type::parts`, `Type::map` and `Type::same_shape`, so
  unification and substitution only need teaching once.
- Type parameters are the first kind of parameter a `Scheme` can have. A
  generic `map` should panic exactly when the `f` it's given does, which is
  *effect polymorphism*: `fn map<T, U, e>(o: Option<T>, f: fn(T) -> U ! e)
  -> Option<U> ! e`, with `e` instantiated at each call just like `T`. Effects
  don't change a value's size or representation, so they can be checked and
  then erased rather than monomorphised.

## Editors

While checking, the checker now records what it learns about every name
(`check::Info`): each variable's type where it's bound and used, what each
name refers to, and the type arguments of each use of something generic. `t
lsp` answers from it: hovering shows a type (`fn id::<int>: fn(int) ->
int`), go to definition jumps to where a variable, function, type, field or
variant is declared, and inlay hints show the types inference found (`let n:
int = ..`) and the type arguments it picked (`id::<int>(1)`). Monomorphisation
reads the same table.

## Design decisions worth noticing

- **Explicit type parameters, rank 1.** `<T>` on items only (`fn`s, nested
  or not, structs and enums). See
  *System F, Hindley–Milner, and what T does*.
- **Type arguments are inferred, written with a turbofish, or some of
  each**: `_` in `::<..>` means "infer this part", as in
  `second::<bool, _>(..)` or `id::<Option<_>>(..)`. `Checker::type_arg`
  renames each `_` to a type parameter no program can name, resolves the
  type as usual, and puts a fresh variable in its place. It's only
  allowed in type arguments, which monomorphisation drops; see the
  exercises.
- **The checker's opinion of a type parameter is fixed.** `Type::Param`
  isn't a variable; the body can't learn anything about it. A *rigid* or
  *skolem* variable, in the jargon.
- **Copies are named `id::<int>`**: Rust's syntax for explicit type
  arguments, which is what using a copy amounts to. Nothing in a source
  program can be *declared* with that name, so copies can't clash with
  anything, but it also means `t mono` output with copies in it doesn't
  parse. For a program without generics it's the program itself, reformatted,
  and the tests check that it parses back to the same thing.
- **Every copy is checked again.** That's cheap here and catches the one
  non-parametric rule. It also checks the pass itself: if monomorphisation
  ever made an ill-typed program, that would show up as an internal error.
- **Copies are made only where something uses them.** A generic function
  nothing calls is never copied, and dead-code warnings still apply to it
  (`warn_generic_dead_code.t`).

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `T: Eq`, `==` on a `T` | bounds are traits | 6 |
| generic closures, `let`-generalisation | closures are values; see above | — |
| `Option<T>` holding itself, lists, trees | needs pointers | 5 |
| returning a closure through a generic | lifetimes | 5 |
| higher-rank types, `fn(∀T. fn(T) -> T)` | can't be monomorphised | — |
| `fn map<T, U, e>` | effect polymorphism | 7 |

## Exercises

1. Write down every function `fn<T>(x: T, y: T) -> T` could be in T. Why
   can't it add them, and why can't it compare them?
2. Monomorphise `tests/t/generics.t` by hand, then check with `t mono`.
   How many copies of `Option` are there, and why that many?
3. Allow `_` in `let` annotations too, as in `let x: Option<_> = ..`.
   The checker part is easy (`Checker::type_arg`). Why does
   monomorphisation then need the checker's help, when it can resolve
   every other annotation by itself?
4. Rewrite `nest` from *Polymorphic recursion* so that it's allowed, by
   counting the levels instead of wrapping the value in them. What did that
   cost?
5. With `struct Box<T> { x: T }`, draw the parameter graph for
   `fn f<A, B>(a: A, b: B) { f(b, Box { x: a }) }`. Does it have a cycle
   that grows? Check with the checker.
6. Boxing instead: suppose every value of a type parameter were a pointer.
   What would the interpreter need to change? What would a compiler?
7. Make `let` generalise, as in exercise 4 of step 3, and monomorphise it
   by copying the `let` once per type it's used at:
   `let id = |x| x; id(1); id(true)` becomes two `let`s. Now try
   `let id = { print(1); |x| x };`. What goes wrong, and how does ML's
   *value restriction* avoid it?
8. Why is it fine for `apply<T>(f: fn(T) -> T, x: T)` to be called with a
   closure that captures, when `id` isn't?

## Reading

- *Types and Programming Languages* (Pierce), chapters 22 (let-polymorphism),
  23 (System F) and 25.
- Philip Wadler, *Theorems for free!*, FPCA 1989. Parametricity.
- Luis Damas and Robin Milner, *Principal type-schemes for functional
  programs*, POPL 1982.
- Alan Mycroft, *Polymorphic type schemes and recursive definitions*, 1984.
- Joe Wells, *Typability and type checking in System F are equivalent and
  undecidable*, 1994/1999.
- ECMA-335 (the .NET CLI spec), on generic types that would expand forever
  ("expansive" cycles).
- *The Rust Reference* on generic parameters, and the rustc dev guide's
  chapter on monomorphization.
