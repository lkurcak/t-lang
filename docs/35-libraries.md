# Step 35: the libraries a package uses

Code: `Loader::manifest` in `src/load.rs` (reading `libraries`),
`Package::libraries` and `Manifest` in `src/ast.rs`, `Modules::unlisted`
in `src/check/modules.rs` (what a path can start with), and
`Checker::unused_dependencies` in `src/check.rs`.

Until now every package had core, alloc and std, whether it used them or
not. So nothing said which libraries a package needs, and nothing could:
a library of hash maps and a program that prints looked the same. That
matters as soon as there's a machine without an operating system
(docs/39, to come): std can't be there, and alloc only if the program
brings an allocator. A package now says which built-in libraries it uses,
in its `t.toml`:

```text
libraries = ["alloc", "std"]

[dependencies]
hashmap = { path = "../hashmap" }
```

Core is every package's, and isn't listed. A package with a `t.toml`
has the libraries it lists, and no others: with no `libraries`, it has
core alone. A package with no `t.toml`, as every single-file program,
has all three, as before.

## Naming a library

A path that starts with a library the package doesn't list is an error,
wherever it is, in a `use` or not:

```text
error: this package's `t.toml` doesn't list the `std` library in its
       `libraries`
 --> main.t:1:13
  |
1 | fn main(os: std::Os) {}
  |             ^^^
  = help: list `std` in `libraries`
```

Its fix lists it, in the `t.toml`. That's the one rule: there's nothing
else a library adds to a package. The prelude is core's, every library's
impls are there for every package's types (as they must be: a
dependency's values carry them), and the names `core`, `alloc` and `std`
are reserved in every package, listed or not, so no module or dependency
can take them.

Listing isn't passed on, as dependencies aren't (docs/26-packages.md):
std uses alloc, but a package that lists std and names
`alloc::string::String` must list alloc too. It can still use the
`String`s std gives it, as it can use a dependency's types without
naming the dependency's dependencies.

A library or package that a `t.toml` lists and nothing names is a
warning (`unused_dependencies`), at its entry in the `t.toml`: the
package doesn't need it. No attribute can be on a `t.toml`, so the
warning can't be allowed; taking the entry out is the fix. As for every
warning, a dependency's are its own.

The manifest checks what it lists:

```text
error: every package has the core library; `libraries` lists only
       `alloc` and `std`
error: no library called `io` is built into T; `libraries` lists only
       `alloc` and `std`
error: `std` is already in `libraries`
```

## Why it's sound

This step adds no rule the type system leans on: what's checked is what
was checked before, minus programs that name a library they don't list.
What it gives is facts about packages, which later steps need.

- **L1, local.** What a package can name comes from its own `t.toml`
  alone. Its dependencies list their own, so adding one never changes
  what this package can name (and never breaks it: the goal of
  docs/29-closed-and-sealed-traits.md).
- **L2, listing changes nothing else.** The prelude, the reserved names
  and the impls are the same whatever is listed, so listing a library
  only turns paths that were errors into paths that resolve. A program
  that checks means the same with a longer list.
- **L3, what a program reaches.** An item is reached by its name, or
  through a value of a type, or an impl. By the orphan rule, an impl is
  in its trait's package or its type's, and a type or trait is reached
  by its name somewhere. So the code a program can run is in the
  libraries that some package of it lists, plus what the target itself
  calls: on an operating system, `std::rt::report` for the error `main`
  gives back. A machine that has those libraries can run the program;
  that's the check a target will make (step 39). The test
  `pkg_libraries` is a program that lists no library at all, and
  prints, through a dependency that lists std.

## Design decisions worth noticing

- **A list, not dependencies.** The libraries could have been entries in
  `[dependencies]`, `std = { builtin = true }`: one table of what a
  package names. But they come from nowhere and have no version, and a
  plain list reads as what it is.
- **A `t.toml` says everything; no `t.toml`, everything.** A single file
  is for trying something out, and gets all three. A package with a
  manifest has said what it is, and a library that lists only what it
  uses is one more machines can use, without its author deciding to be
  portable. (Rust goes the other way: std unless `#![no_std]`.)
- **No target yet.** Which machine a program is for will say which
  libraries are there, but there's only one kind of machine so far, and
  whether the target is a manifest's or the build's to say is a question
  for step 39. Checking doesn't depend on it: a program is checked once
  for every machine (docs/11-integers.md).

## Deliberately missing (coming later)

- Targets, and checking that a program's libraries are on its target.
- A list of the libraries a dependency uses, shown to its dependents.
- Allowing `unused_dependencies` for one entry.

## Exercises

1. A library lists std only to name `std::io::Error` in one function.
   What does it lose, for its users, and how could it avoid naming it?
2. Why must the names `alloc` and `std` stay reserved in a package that
   doesn't list them? Write a program whose meaning would change when
   its `t.toml` listed `alloc`, if they weren't.
3. L3 leans on the orphan rule. Write the impl, in std, that would let a
   program reach std's code while no package of it lists std, and find
   the error the orphan rule gives it.
4. Should `t.toml`'s `libraries` be sorted, as `[dependencies]` is when
   read? What would change if a later library depended on the order?

## Reading

- The Rust Reference, *Preludes* and the `no_std` attribute: std by
  default, and the opt-out that makes a crate usable without an
  operating system.
- Miller, *Robust Composition* (2006): authority as what a component is
  given, which std's `Os` already is; a package's libraries are what it
  can name, the static half of the same idea.
