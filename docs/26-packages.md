# Step 26: packages

Code: `Loader` in `src/load.rs` (reading `t.toml`s and the packages they
name, and the chain rule), `Package` and `Program::split_off` in
`src/ast.rs`, `check::check` (each package checked in turn), and
`Modules::package_named` in `src/check/modules.rs` (what a path can start
with).

Until now a program had three packages it didn't choose, core, alloc and
std, and its own. Now it can depend on a library someone wrote, by where
it is on disk. A *library* is a directory whose first file is `lib.t`; a
program's or a library's `t.toml`, beside its first file, says what it
depends on:

```text
app/
  t.toml        [dependencies]
                counter = { path = "../counter" }
  main.t
counter/
  lib.t
  show.t        (module `show`, as in docs/06d.1-files.md)
```

`counter/lib.t` is a package's root, as `main.t` is a program's. What it
exports is what other packages see of it (docs/24-visibility.md):

```rust
//! A counter that only counts up: its field is its own.

/// Counts up from zero.
#[derive(Discard)]
export struct Counter { n: i64 }

impl Counter {
    export fn new() -> Counter { Counter { n: 0 } }
    export fn up(&mut self) with panic {
        self.n = self.n + 1;
    }
    export fn get(&self) -> i64 { self.n }
}
```

In `app`, the name the `t.toml` gives it starts paths, in every module,
as `core` does:

```text
use counter::Counter;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = Counter::new();
    c.up();
    out.line(&c.get())?; // 1
    Result::Ok(())
}
```

`t check counter/lib.t` checks the library on its own, with no `main`:
what it exports is what makes the rest of it live, for dead code. `t
counter/lib.t` says there's nothing to run.

## The manifest

A `t.toml` has one table, `[dependencies]`, of a name for each package and
where it is: `name = { path = "dir" }`, the path from the `t.toml`'s own
directory. That's all it can say for now, and anything else is an error,
rather than something quietly ignored: a `version`, a `[package]` table.
A package with no dependencies needs no `t.toml`.

Only a package that starts in `main.t` or `lib.t` has one, as only those
have modules in files; a directory has one or the other, not both. A
module in a file can't be called `lib`, as it can't be called `main`: the
file `lib.t` is where a library starts.

The built-in libraries are still there for every package without asking:
`std`'s I/O needs the `Os` that only `main` is given (docs/19-io.md), so a
library that's handed nothing can't do I/O behind its dependents' backs.

## Names and identity

A package is a directory. It's read once, whichever packages depend on it,
so a type of its is one type everywhere: `app` can depend on `shapes` and
`geometry` both, where `shapes` depends on `geometry` too and has `export
use geometry::Point;`, and `shapes::Point` is `geometry::Point`.

Its name is what its dependents call it, and each package has one name in
a program, which the symbols of its items start with: `counter::Counter`.
So every dependent must call a package the same, and two packages can't
have the same name:

```text
error: the package in `geometry` is called `geometry` here but `geo` in
       `t.toml`; call it the same everywhere
error: two packages are called `x`: the one in `inner` and the one in
       `outer`
```

A name must be a name in T, and not `core`, `alloc` or `std`; and a module
can't have the name of a package its package depends on, which would make
`counter::f` mean two things.

What a package depends on is its own business: `app` sees `shapes`, not
`geometry`, unless its own `t.toml` names it. A package can pass on what
it wants its dependents to see with `export use`.

## Order, and the chain rule

Each package is read after those it depends on, and checked after them,
against what checking them left, as step 14 checks a program after the
libraries (docs/14-packages.md). Packages can't depend on each other, so
there's always such an order:

```text
error: packages can't depend on each other: `a` depends on `b`, which
       depends on `a`
```

Step 14's argument that a package checks the same alone as with any
program after it (M1, M2) is about a *chain*: each package after all those
it depends on, and depending on all those before it. Some of the rules
are about the whole program: that proving what implements what ends (the
impl-bound termination of step 7.1, a graph of traits over every impl),
that associated types are finite, that there's no polymorphic recursion
through impls. A chain checks them a package at a time, as a cycle that's
new goes through one of the new package's items, which is at fault.

Two packages that don't know each other break that. Over a package `P`
with traits `A` and `B`, `Q` writes

```text
impl<I: Iterator> A for W<I> where I::Item: B {}
```

and `R` writes

```text
impl<T: A> B for V<T> {}
```

Each is fine alone: `Q`'s graph has `A → B`, `R`'s `B → A`. Together they
have a cycle, and a real one: with an `X` whose `Item` is `V<W<X>>`,
proving `W<X>: A` needs `V<W<X>>: B`, which needs `W<X>: A` again.
Neither package is wrong; the program that puts them together is. Checking
them in one order would blame whichever came second, for something it
didn't do.

So for now, the packages of a program must form a chain: each must depend,
directly or not, on the one before it. Then the one order there is checks
each package against exactly the packages it depends on. The import that
puts two independent packages together is an error:

```text
error: `left` and `right` don't depend on one another, which packages of
       one program must, for now: ...
```

A diamond along a chain, like `app`, `shapes` and `geometry` above, is
fine. The rule is a restriction to relax: each package's whole-program
rules checked over only what it depends on, and checked again where two
independent packages meet, with an error there saying they can't be used
together.

The program is read whole before anything is checked, so as each package
is checked, the items of those after it are set aside
(`Program::split_off`): step 14's passes take a package's items to be
those after its start, which is true when nothing after it has been read
yet.

## Why it's sound

- **P1, identity.** A package is read once per program, by its directory,
  and has one name, so each of its items has one symbol and one meaning.
  Without it, two copies of `geometry` would make two types
  `geometry::Point`, the same by name and not the same.
- **P2, modularity.** Under the chain rule, a package is checked against
  exactly the packages it depends on, so it checks the same alone as in
  any program: its acceptance is final, and an error that only shows up
  with a dependent is the dependent's. The test
  `dependencies_check_as_they_do_alone` checks every dependency in
  `tests/t` alone.
- **P3, reading ends.** Each directory is read once, and a package that
  would need itself is an error, so there are as many packages as
  directories named, and reading them ends.
- **P4, nothing new to trust.** What crosses from one package to another
  is what step 24 lets cross: exported items, written effects, the orphan
  rule per package. A user's package is a package as a built-in library
  is, but with none of their privileges: no intrinsics, no impls for
  `str` or slices.

## Design decisions worth noticing

- **A manifest, not `import`.** What a package depends on could be said
  in its source, as `import hashmap from "../hashmap";`, which would make
  `import` the dual of `export`. The manifest keeps where packages are,
  and later which versions, out of the language: moving a dependency
  doesn't change a line of code.
- **The dependent names the package.** No package has a global name to
  clash on; what its symbols start with is what the program calls it,
  and the one-name rule keeps that one thing.
- **A chain, for now.** It's the shape the soundness argument of step 14
  covers unchanged, and every program so far has it: tcalc will depend on
  one library.
- **Dependencies' warnings are theirs.** A program shows its own
  warnings; a library's own `t check` shows its.

## Deliberately missing (coming later)

- Versions, a registry, a lockfile.
- Packages that don't know each other in one program (the chain rule).
- A dependent renaming a package, or two versions of one in a program.
- Keeping a checked package between runs, or across programs in the LSP.
- `t new`, workspaces, dependencies only for tests.

## Exercises

1. Write `Q`, `R` and a program for the cycle above, with `P` as a
   package of their own. Which error do you get, and where? What would
   the error be if `R` depended on `Q`?
2. A package can be called something else by each of two programs. Which
   messages show its name, and could a program tell, from what it prints,
   what its dependency is called elsewhere?
3. Why can't a module in `app` be called `counter`, when a module in
   `counter` could be? What goes wrong in `Modules::modules` if it could?
4. Make a dependency's warnings show in its dependent. Which golden test
   fails? Which warnings would be noise, and which, if any, would be
   useful there?
5. Relax the chain rule as described above. Which of step 14's
   whole-program rules need checking again where two packages meet, and
   which, like overlap, can't fail there because of the orphan rule?

## Reading

- Cardelli, *Program Fragments, Linking, and Modularization* (1997):
  linking as needing nothing more than each fragment's checking, which
  is what the chain rule keeps true here.
- Kilpatrick, Dreyer, Peyton Jones and Marlow, *Backpack: Retrofitting
  Haskell with Interfaces* (2014): packages as things to put together,
  and what can go wrong when they are.
- Rust RFC 1023, *Rebalancing Coherence* (2015): the orphan rule as what
  makes any two crates' impls fit together, which T's termination rule
  doesn't do yet.
- Cox, *Minimal Version Selection* (2018): one version of each package in
  a build, chosen from what the manifests say: where versions could go
  next.
