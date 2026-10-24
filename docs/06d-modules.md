# Step 6d: modules

Code: `src/check/modules.rs` (what each path names, `use`s, and which
types are private in public signatures), `src/check/names.rs` (type names
in each module's namespace), `src/borrowck/moves.rs` (`abstraction`: who
may take a struct apart), and `src/check/traits/` (`derive`, for
written bounds, and `lookup_method`, for which traits a call sees).

```rust
mod geometry {
    #[derive(Copy, Discard)]
    pub struct Point { pub x: i64, pub y: i64 }

    pub fn dist(a: Point, b: Point) -> i64 with panic { abs(a.x - b.x) + abs(a.y - b.y) }

    fn abs(n: i64) -> i64 with panic {
        if n < 0 { -n } else { n }
    }
}

use geometry::{Point, dist};

fn main() with panic {
    let p = Point { x: 1, y: 2 };
    print(dist(p, Point { x: 4, y: 6 })); // expect: 7
    print(geometry::dist(p, p));          // expect: 0
}
```

A module is a namespace: `mod geometry { .. }` holds items as a file does,
and `geometry::dist` names one of them from outside. Until now everything
in a program saw everything else. A library needs a place of its own,
where its names don't clash with yours, and parts that only it can see, so
it can change them without breaking you. That's what this step adds, and
the rest of the standard library (step 6e on) needs it.

## Namespaces

Each module has two namespaces. *Types* holds its structs, enums, traits
and modules, and *values* its functions, so a module `main` and a function
`main` don't clash. A name in a module means what that module declares or
imports, and nothing else: the modules around it aren't in scope.

```rust,error
fn helper() -> i64 { 1 }

mod m {
    pub fn f() -> i64 { helper() }
}

fn main() {
    print(m::f());
}
// error: unknown variable `helper`
```

The crate root's `helper` is `super::helper` here, or `crate::helper` from
anywhere. Such a module is *hermetic*: what it means doesn't depend on where
it sits, so moving it elsewhere, or into a file of its own (see
[docs/06d.1-files.md](06d.1-files.md)), changes nothing but the paths that
lead to it.

A path starts at a name in scope, or at `crate` (the root), `super` (the
module around this one) or `self` (this one), and goes through modules to
an item. A `use` brings an item's name into a module, as if it were
declared there, maybe under another one:

```rust
mod shapes {
    pub mod round {
        pub fn area(r: i64) -> i64 with panic { 3 * r * r }
    }
}

use shapes::round::area as circle;

fn main() with panic {
    print(circle(2)); // expect: 12
}
```

What a path names is decided before checking, and doesn't depend on the
order of anything. There are no glob imports (`use a::*`), so a name in a
module is one item, or one `use`, in each namespace, and two of the same
name is an error. A `use` may go through another `use`, so they're
resolved as they're needed, and one that needs itself, through others, is
an error, so resolving ends. With that, what each path names is a
function of the program (lemma M1).

## Seeing and not seeing

An item is visible in its module and in the modules inside that. One that's
`pub` is visible wherever its module is. A struct's fields and an inherent
impl's methods are each `pub` or not. An enum's variants are public
wherever the enum is: a variant that couldn't be seen would make every
`match` outside the module non-exhaustive.

Each segment of a path must be visible where it's written, so a `pub`
function in a private module can't be reached from outside the module
around it. A `pub use` makes a name public where it is, but can't make
public what isn't:

```rust,error
mod a {
    fn f() -> i64 { 1 }

    pub use self::f as g;
}

fn main() {
    print(a::g());
}
// error: `f` is private, so `pub use` can't make it public
```

A public item can't mention a private type or trait in what it takes,
gives, or needs of its type parameters: code that can call it couldn't
write down its signature.

```rust,error
mod m {
    #[derive(Discard)]
    struct Hidden { n: i64 }

    pub fn make() -> Hidden { Hidden { n: 1 } }
}

fn main() {}
// error: `make` is public, but its signature names `Hidden`, which is private to `m`
```

That isn't needed for soundness: a value of a type you can't name is like
one of an opaque type (docs/06c.1-opaque-types.md). It's a restriction that
can be lifted later, to a warning, as Rust did, without breaking anything.

## Abstract types

A struct with a field you can't see is *abstract* where you are: you can't
make one, as you can't give that field, nor take one apart. So every value
of it was made by its module, which can keep promises about it that the
type alone doesn't, like a `List` whose length is at most its capacity.
Mitchell and Plotkin's *Abstract types have existential type* is the
theory: outside its module, a struct with private fields is an existential
type, the module's functions its only operations, and Reynolds'
*representation independence* says its fields can then change without
anything outside noticing.

With linear types (docs/06a.1-linear-types.md) a module can make promises
about how a value ends, too. An `Fd` that isn't `Discard` must be used up,
and only `close` can take one apart, so every file that's opened is closed:

```rust
mod fs {
    pub struct Fd { pub id: i64, raw: i64 }

    pub fn open(id: i64) -> Fd with panic { Fd { id, raw: id * 10 } }

    pub fn close(fd: Fd) -> i64 with panic {
        let Fd { id, raw } = fd;
        id + raw
    }
}

fn main() with panic {
    let fd = fs::open(2);
    print(fd.id);         // expect: 2
    print(fs::close(fd)); // expect: 22
}
```

But a pub field is a way in. Moving it out, or matching the `Fd` by value
with `fs::Fd { id, .. }`, would take the `Fd` apart and throw the private
rest away, closing nothing: the `raw` field is an `i64`, which may be
thrown away, so the linear check alone wouldn't stop it. So outside its
module, a struct with private fields can't be taken apart at all, unless
it's `Discard`, which is its module saying that throwing it away is fine.
Reading a pub field through a place, by copying it or borrowing it, takes
nothing apart:

```rust,error
mod fs {
    #[derive(Discard)]
    pub struct Id { pub n: i64 }

    pub struct Fd { pub id: Id, raw: i64 }

    pub fn open(n: i64) -> Fd { Fd { id: Id { n }, raw: n } }

    pub fn close(fd: Fd) -> i64 with panic {
        let Fd { id, raw } = fd;
        id.n + raw
    }
}

fn main() {
    let fd = fs::open(1);
    let id = fd.id;
    print(id.n);
}
// error: can't take `Fd` apart here: it has private fields and isn't `Discard`, so only `fs` can
```

That's Rust's rule that nothing can be moved out of a type with a
destructor, in another form. The borrow checker checks it
(`moves::abstraction`), as it knows what's moved: a move out of a field of
such a struct, and a pattern that takes one apart by value, both of which
it sees as statements of the body. That's lemma M2: outside its module, a
struct that isn't `Discard` and has private fields is only ever moved,
borrowed, read through, and handed back.

## Which trait

`x.show()` looks for `show` among the traits in scope where it's written:
those declared in the module and those it imports, and the built-in ones.
A trait in some other module doesn't count, so adding one can't make a
call here ambiguous, or mean something else. `Show::show(x)` names the
trait, and needs no import. A type parameter's bounds are in scope
wherever it is.

```rust,error
mod show {
    pub trait Show {
        fn show(&self) -> i64;
    }

    impl Show for i64 {
        fn show(&self) -> i64 { *self }
    }
}

fn main() {
    print(1i64.show());
}
// error: `i64` has no method `show` in scope; trait `show::Show` has one: import it with `use show::Show;`
```

What's in scope picks which trait a call means, never which impl: that's
still the one impl of the trait for the type, wherever it is (lemma M4).

## Coherence and orphans

An impl isn't scoped. Wherever it's written, it's the impl of its trait for
its type everywhere, and two that overlap are an error even in different
modules. Scala's implicits are the other way: which instance is used depends
on what's in scope, so a `Set` built where one ordering is in scope and
searched where another is breaks. A global impl is what makes the
dictionary-passing and monomorphising semantics agree (docs/06-traits.md),
and that's unchanged (lemma M3).

What keeps impls from overlapping when separately built packages are put
together is an *orphan rule*: an impl must be in the package of its trait,
or of its type. Within one package, overlap is checked across all of it, so
the rule is per package, not per module, and impls can go in any module.
For now the only other package is what's built in, and the rule is the one
T already had: a built-in trait like `Eq` can't be implemented for a
built-in type like `i64`. With `core` (step 6e), it's a real package, and
since step 17.1 `Eq` is one of its items.

## Derives at the boundary

A derive works out what it needs from the fields: `#[derive(Discard)]` on a
struct holding a `Box<T>` needs `T: Discard`. If the field is private, that
lets it decide part of the public interface, and changing the field could
break code outside. So a public struct with private fields says in each
derive what it needs, and the checker checks that it's enough:

```rust
mod cells {
    use alloc::boxed::Box;

    #[derive(Discard where T: Discard)]
    pub struct Cell<T> { v: Box<T> }

    pub fn cell<T>(v: T) -> Cell<T> with {alloc, panic} { Cell { v: Box::new(v) } }
}

fn main() with {alloc, panic} {
    // A `Cell<i64>` is `Discard`, so it can be thrown away.
    let _c = cells::cell(5i64);
}
```

Saying more than the fields need is allowed: a struct can promise less
than it gives now, to have room to change. Writing nothing promises the
derive needs nothing, and is an error if it does, with a fix:

```rust,error
mod cells {
    use alloc::boxed::Box;

    #[derive(Discard)]
    pub struct Cell<T> { v: Box<T> }
}

fn main() {}
// error: `Cell` is public with private fields, so its `Discard` must say what it needs: `where T: Discard`
```

A struct whose fields are all public keeps working its bounds out, as
they're worked out from what everyone can see already (lemma M5).

## Running it

Modules are only names. An item in a module gets a symbol made of its
module's path and its name, `geometry::dist`, given before those of nested
functions and local types, which get a `#2` after if they'd be the same.
From there on every function and type is named by its symbol, so lifting,
monomorphisation and the interpreter don't know modules exist.

## Design decisions worth noticing

- **Hermetic, as Rust is since 2018.** A module that saw the names around
  it would mean something different wherever it sat.
- **`Discard` is consent.** A struct with private fields can be taken apart
  outside its module exactly when it may be thrown away, so the rule costs
  nothing for plain data, and protects resources.
- **Per-package orphans.** A per-module rule would be stricter, and could be
  relaxed later, but it adds no soundness, and costs a lot.
- **Inherent impls in their type's module.** All of a type's own methods,
  and all the code that sees its private fields, are in one place.
- **No glob imports.** A `use a::*` that brings in a name another module
  adds later can change what code means; Rust's rules for that are subtle.
  It can come later.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| `mod m;` from a file `m.t` | the CLI and the editor support need to know files | 6d.1 ✅ |
| `use a::*` | shadowing rules for names globs bring in | — |
| `pub(crate)`, `pub(super)` | `pub` and private are enough so far | — |
| `use` in a function | items in blocks are enough so far | — |
| packages, and `core` | done: docs/06e-core.md | 6e ✅ |
| private types in public signatures as a warning | an error can always be relaxed | — |

## Exercises

1. Write a module `counter` with a struct `Counter` whose count only goes
   up, and functions to make one, add one, and read it. What stops code
   outside from making a `Counter` whose count is 5?
2. Delete the `!b.cx.is_discard(&t)` in `moves::abstraction` and run the
   tests. Which fails, and what would that program be allowed to do?
3. Why are an enum's variants public wherever it is, instead of each being
   `pub` or not? What would `match` need to know if they weren't?
4. Two modules each declare a trait with a method `size`, and `i64`
   implements both. Which calls of `1.size()` are fine, and which aren't?
5. Write a program whose meaning would change if impls were scoped: one
   where which impl of `Eq` is used depends on which module a function is
   in.
6. Why does a derive on a public struct whose fields are all public not need
   a `where`, when one with a private field does?

## Reading

- John Mitchell and Gordon Plotkin, *Abstract types have existential type*
  (1988).
- John Reynolds, *Types, abstraction and parametric polymorphism* (1983),
  on representation independence.
- Benjamin Pierce, *Types and Programming Languages*, chapter 24
  (existential types).
- Derek Dreyer, Robert Harper and Manuel Chakravarty, *Modular type classes*
  (2007), and Edward Kmett, *Type classes vs. the world* (2015), on why
  instances are global.
- The Rust reference, *Paths*, *Use declarations* and *Visibility and
  privacy*; Rust RFC 1023 (the orphan rules) and RFC 2126 (paths since
  2018).
