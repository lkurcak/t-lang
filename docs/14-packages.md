# Step 14: checking packages separately

Code: `check.rs` (`libraries`, `Checker::package`), `load.rs`, and the
passes of `check/`, each of which now looks only at the package being
checked. Nothing a program means changed. It checks faster, and one
error is now reported somewhere else.

A program has three packages: the core library, the alloc library and
its own crate, each depending on the ones before it. Until now they
were read into one program and checked together, so every program
checked core and alloc again. Now each package is checked once, on its
own, against what checking the packages before it left:

1. core, alone;
2. alloc, against core;
3. the program, against both.

The first two are the same for every program, so they're done once per
run of the compiler. What their checking leaves (their items with paths
resolved, their types, impls and signatures, and what was learned about
their bodies) is kept, and each program's checking starts from a copy of
it. Lifting, monomorphisation and the interpreter still see the whole
program: a generic function's body is part of its library's interface,
since a copy of it is made for each type it's used at.

```rust
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let s = (0..10i64).filter(|x| *x % 2 == 0).fold(0, |a, x| a + x);
    out.line(&s)?; // expect: 20
    Result::Ok(())
}
```

This program uses `Range`, `Iterator`, `Filter` and `fold` from core. None
of them is checked again for it.

## Why it means the same

Checking the packages one after another gives the same answers as
checking them together only if no rule needs to see a later package to
judge an earlier one. That's three claims.

**M1. A package checks the same alone as with any package after it.**
Core is checked before any program exists, so it can't depend on one.
What has to be shown is that a later package's checking never *changes*
what's known of an earlier one, as a copy is all it gets. The orphan rule
(docs/06e-core.md) is what makes it so: a program's impls are of its own
traits or for its own types, so whether a core type implements a core
trait, and what a projection on it stands for, is settled by core. A
test checks every program in `tests/t` and compares what's known of the
libraries before and after:
`checking_a_program_leaves_the_libraries_as_they_were`.

**M2. A rule about the whole program splits into one about each
package.** Some rules aren't about one item but about all of them:

- *No cycles*: impls whose bounds lead back to their own trait
  (`termination`, `bound_termination`, `assoc_termination`), generic
  functions that use themselves at bigger types, through impls too
  (polymorphic recursion, docs/04-generics.md), opaque types that hold
  themselves, structs that hold themselves.
- *No overlap*: two impls a type could use (coherence).

Each is a property of a graph of the program's items, or of pairs of
them. An earlier package's own graph has no cycle and its own pairs
don't overlap, as it checked. So a cycle in the graph of earlier
packages and the new one has at least one edge of the new one, and an
overlapping pair has at least one impl of it. That edge, or that impl,
is where the error is reported.

**M3. What's inferred is inferred in one package.** Closures' captures
and results, what an opaque type stands for, the impl parameters a
bound decides (docs/07.1-impl-parameters.md) and refinement facts are
all worked out within one function or one impl, and an earlier
package's are settled before a later one starts.

## Whose fault it is

M2 says where an error goes. A library that checked stays checked, so if
a rule now fails, it's because of the package being checked, and its
error is there. Before this step, one rule could put a program's error
in core, because it reported the first impl it found in an order where
core's came first:

```rust,error
// Core's `impl Iterator for Filter<I, P>` needs `I::Item: Discard`, which
// isn't a part of `Filter<I, P>`. That's fine as long as proving
// `Discard` can't lead back to `Iterator`, and in core alone it can't.
struct W<T> { t: T }

impl<T: Iterator + Discard> Discard for W<T> {
    fn discard(self) {}
}
// error: an impl of `Discard` may only need its type parameters to be `Discard`
```

(Since step 17 such an impl is an error of its own, and since step 20
that's the error: an impl of `Discard` may say how a value is thrown
away, but not need more than a derived one.)

Proving that a type implements `Iterator` could then go on forever, as
docs/07.1-impl-parameters.md explains for one package. Until now the
error pointed at `Filter`'s impl, in `<core>/iter.t`, which the program
can't change. This step made it point at `W`'s impl, the one that made
the cycle, naming `Filter`'s as the impl it makes loop.

Since step 17 (docs/17-retrospective-1.md) this impl can't be written at
all: `Discard` is only derived, and a derived `Discard` needs only
`Discard`, so proving it never leads to another trait. The rule stays,
for cycles through the impls of traits a program declares.

The same goes for polymorphic recursion. If a cycle that makes types
bigger runs through library functions, the error is at a use in the
program's own code. That's one that makes types bigger if the program
has one, and otherwise any use of the program's that's on the cycle.

## What's kept, and where

Each package's items come after the packages before it, in every list:
modules, `use`s, types, traits, impls and functions, and in the source
text, so its spans come after theirs too. Functions are numbered package
by package (`Program::all_fns`). So what checking the earlier packages
built, which is indexed by those numbers, is a prefix of what checking
the next one builds, and the next one's checking adds to the end. Each
pass looks only at what comes after `Counts`, where the package starts.

The crate root, module 0, is the exception. It's there, empty, before the
libraries are read, and the program's first file fills it.

`load` starts from the libraries as checking left them, with their paths
resolved. So tools that only read a program (`t fmt` doesn't, but
renaming and the language server do) get the libraries' checked text
for free.

## How much faster

Checking `fn main() {}` in a process that has checked a program before
takes about 4 ms, down from 29 ms, as only the program is checked. The
first check in a process still checks the libraries, so a single
`t check` takes as long as before. The test suite, which checks over a
thousand programs in one process, takes half as long, and so does the
language server after its first check.

## Design decisions worth noticing

- **A copy of the checker's state is the interface.** The alternative is
  a separate value with only what a later package may see: signatures,
  impls, generic bodies. That boundary would be enforced by the type
  system rather than by a test. But nearly every pass would have to read
  from it instead of from its own tables, and a generic body is part of
  the interface anyway. The test of M1 is what keeps the copy honest.
- **Core and alloc are two packages.** One could check them together,
  as they're always both there. Checking alloc against core exercises the
  same machinery between two libraries as between a library and a
  program, and catches a mistake on that boundary.
- **The newest package is at fault.** That's what makes an accepted
  library stay accepted. It doesn't make more programs check: the
  program above is still rejected, now in the right place. A termination
  argument that let it check would need a measure that holds across
  packages, which is research.

## Deliberately missing (coming later)

- Packages a program writes itself, with a manifest: there are only the
  built-in ones.
- Keeping a checked package on disk, between runs.
- Rules for what a new version of a package may change: an interface
  gives them, as strengthening a refinement, adding an effect or an impl
  can break a package after it.
- Not lifting or monomorphising the libraries again for each program.

## Exercises

1. Which pass would go wrong if a program's impl could be for a core type
   and a core trait? Write such an impl in a test and see what the orphan
   rule says.
2. A blanket impl in core, `impl<T: A> B for T`, and one in a program,
   `impl<T: B> A for W<T>`: is that a cycle in the sense of
   `termination`, or does the Paterson condition already see to it? And
   with a blanket `impl<T: B> A for T` in the program?
3. Why can't a library's associated type get to a program's impl through
   a projection? What would have to change in the language for it to be
   able to, and which check would then have to look at the libraries'
   impls again?
4. Make `polymorphic_recursion` report the first use on a cycle that
   makes types bigger, whichever package it's in, as before this step.
   Which test fails? Why can't a program in `tests/t` show it with
   today's core?
5. `checking_a_program_leaves_the_libraries_as_they_were` compares what's
   known of the libraries before and after checking. Make a program's
   checking change one of a library's impls, and see it fail.

## Reading

- Cardelli, *Program Fragments, Linking, and Modularization* (1997):
  separate compilation as checking each fragment against the interfaces
  of those it uses, and linking as needing nothing more.
- Leroy, *Manifest Types, Modules, and Separate Compilation* (1994):
  what an interface must say so a module can be checked without the
  others.
- Rust RFC 1023, *Rebalancing Coherence* (2015): the orphan rule as what
  lets crates be checked one by one, and what adding an impl can break.
