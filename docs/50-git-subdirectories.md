# Step 50: packages in a repository's subdirectory

Code: `Source::Git` and `Loader::inside` in `src/load.rs`, and
`git::is_subdirectory` and `git::name` in `src/load/git.rs`.

Step 27 made a repository's top its package. That's one repository per
library, and a set of small libraries that grow together (a hash map, a
grid, a graph) wants one repository with a directory each. Now a git
dependency can say which directory its package is in:

```text
[dependencies]
hashmap = { git = "https://github.com/lkurcak/t-libs", rev = "92e5...", subdirectory = "hashmap" }
```

The package is that directory of the commit's checkout: its first file is
`lib.t` there, and everything step 26 says about a package holds for it.
Without a `subdirectory`, the package is at the top, as before.

This is deliberately small. It is not a workspace: the repository has no
manifest of its own listing its packages, and nothing builds or tests
them together. That's a design of its own, for later.

## What a subdirectory is

Names separated by `/`, none of them empty, `.` or `..`, and none with a
`\` or a `:`:

```text
error: a `subdirectory` is a directory in the repository, as names
       separated by `/`, as in `"hashmap"` or `"libs/grid"`; not `../g`
```

So a subdirectory is inside the checkout by its spelling alone, and means
the same directory on every system: no `..` leads out, no leading `/`
starts elsewhere, and no `\` or drive letter means one thing on Windows
and another elsewhere. The top of the repository has no subdirectory,
rather than `""` or `"."`: one way to say it.

A `subdirectory` belongs to a `git` dependency. With a `path`, the path
already says the whole way:

```text
error: a `subdirectory` is a directory of a `git` dependency's
       repository, and this has no `git`; a `path` says the whole way to
       its package
```

## One checkout, one rev

Packages from one repository at one `rev` share its checkout, fetched
once. Inside it they depend on each other by `path`, which step 27
already confined to the checkout, so `grid/t.toml` can say
`hashmap = { path = "../hashmap" }`.

Step 27's rule that a program has one version of each repository is
unchanged, and now covers every package in it: a program can't take
`hashmap` from one commit of `t-libs` and `grid` from another.

```text
error: two versions of `https://github.com/lkurcak/t-libs`: `92e5388` in
       `t.toml`, and `ea992bc` here; a program has one version of each
       package, for now
```

Two commits of one repository are two versions of whatever they share:
if `grid` depends on `../hashmap`, a program with `hashmap` from another
commit would have two `Map`s that must be one (lemma P1 of
docs/26-packages.md). Bumping them together is the price; a workspace
with versions per package would be the way to lift it.

In messages, a checkout is named by its repository, as its URL ends,
since it can hold several packages:

```text
type error: expected `i64`, found `bool`
 --> <t-libs@92e5388>/hashmap/lib.t:1:24
```

## Why it's sound

**Lemma S1 (identity).** A package from git is the directory
(url, rev, subdirectory) names in the cache, and a package's identity is
still its directory (docs/26-packages.md, P1). Two entries with the same
three are one package, so the one-name rule of step 26 applies to them;
two subdirectories are two packages that happen to share a checkout.

**Lemma S2 (confinement).** The subdirectory's spelling has no `..` and
no root, so joined to the checkout it is lexically inside it. The only
way out would be a link; git checks links out as files holding their
targets (step 27), and `Loader::inside` checks the real path besides, as
`Loader::path` does for `path` dependencies, since the cache is anyone's
to change.

**Lemma S3 (content).** The subdirectory's files are part of the commit,
so its `rev` pins them, as it pins the top's (step 27, P1').

Loading still ends: there are finitely many directories, each loaded
once, and a cycle among them is caught by its directory, wherever it is.

## Decisions

- **`subdirectory`**, as pip and uv spell it, over the shorter `dir`, or
  `path` beside `git`, which would mean two different things.
- **Labels by repository.** `<t-libs@92e5388>` rather than the first
  package's name, which would show `grid`'s files as
  `<hashmap@92e5388>/grid/lib.t`.
- **One version per repository**, as above.

## Exercises

1. Allow `subdirectory = "."` as a spelling of the top. What would have
   to change for the one-name rule to see it as the same package as one
   with no `subdirectory`? (Hint: what is the identity compared on?)
2. A repository moves `hashmap` from its top into `hashmap/` between two
   commits. Which programs break, and what message do they see?
3. Sketch a workspace: a manifest at the repository's top that lists its
   packages. What would it let a dependent leave out, and what could it
   check that a single package's `t.toml` can't?

## Reading

- pip's documentation, *VCS Support*: the `subdirectory` fragment.
- Cargo's documentation, *Workspaces*: where this goes next, and Cargo's
  search for a package by name in a git repository.
