# Step 27: git dependencies

Code: `Source`, `Loader::path` and `Loader::fetch` in `src/load.rs` (what a
`t.toml` entry names, and where it is), `src/load/git.rs` (fetching and the
cache), and `Sources::label` in `src/source.rs` (how a checkout's files are
shown).

Step 26 let a package depend on another by where it is on disk. That's
enough for a library beside its program, but not for one that lives in a
repository of its own: a program would need a copy of it, or a submodule,
at a known place beside it. Now a `t.toml` can name a commit of a
repository instead:

```text
[dependencies]
hashmap = { git = "https://github.com/lkurcak/t-hashmap", rev = "5f0c..." }
```

The `rev` is the commit's full hash, 40 hex digits (or 64, for a SHA-256
repository), in lower case. The repository's root is the package: its
first file is `lib.t` there, and everything step 26 says about a package
holds for it.

## A commit, and only a commit

A commit's hash is a hash of its files, so a `rev` means the same files
wherever and whenever it's fetched. That makes the manifest its own lock:
there's nothing for a `t.lock` to record that the `t.toml` doesn't say
already, and so there's no lockfile yet.

What isn't allowed follows from that:

```text
error: a `rev` is a commit's full hash, 40 lowercase hex digits (or 64),
       not `0123abc`
error: a dependency has only a `path`, or a `git` and a `rev`, not a
       `branch`: a branch or tag can move, so a `git` dependency is pinned
       by its `rev`, for now
```

A short hash names one commit today and possibly two tomorrow, as the
repository grows. An upper-case one names the same commit as the
lower-case one, and would make one package look like two. A branch or a
tag moves, so it would need a lockfile to say what it meant; those come
with the first source that can move, and `rev` stays valid then, so adding
them won't break a manifest.

## The cache

A commit is checked out once, into the cache:

```text
$T_LANG_CACHE/git/<name>-<hash of the url>/<rev>/
```

`$T_LANG_CACHE` if it's set, else the system's place for caches, in
`t-lang`: `$XDG_CACHE_HOME/t-lang` or `~/.cache/t-lang`,
`~/Library/Caches/t-lang` on macOS, `%LOCALAPPDATA%\t-lang\cache` on
Windows. A checkout that's there is never fetched again, since it can't
have changed: once a program has been checked, it checks with no network.

A checkout is fetched beside where it goes and then moved there whole, so
two `t`s fetching the same commit at once both find all of it or nothing,
and a fetch that fails leaves nothing to be taken for the package.

`t check`, `t run` and the editor all fetch what's missing. In messages, a
checkout's files are shown by the package's name and rev, not by where
the cache happens to be:

```text
type error: expected `i64`, found `bool`
 --> <hashmap@5f0c1a2>/lib.t:1:24
```

## Fetching runs nothing

T has no build scripts, no macros and no code that runs while a program is
checked, and a library can't do I/O unless it's handed the `Os` that only
`main` gets (docs/19-std-io.md). So depending on a package from git trusts
it with nothing but its types. Fetching keeps it that way:

- **git does the fetching**, as the `git` command, so the user's
  credentials, ssh keys and proxies are what reach the repository. It asks
  for just the commit (`fetch --depth 1 <rev>`, which GitHub serves), and
  fetches everything if the server won't, then checks that what's checked
  out is the commit asked for.
- **Only transports that just fetch:** `https://`, `http://`, `ssh://`,
  `git://`, `file://` and `user@host:path`. Not git's `ext::`, which runs a
  command to reach a repository, nor anything git could take for an option:

  ```text
  error: `ext::sh -c ..` isn't a URL T fetches from: it takes `https://`,
         `http://`, `ssh://`, `git://`, `file://` and `user@host:path`
  ```

  The same list is given to git as `GIT_ALLOW_PROTOCOL`, so not even a
  redirect can switch to another transport.
- **No hooks:** not the repository's, which a fetch never installs, and
  not the user's own, which would otherwise run on checkout.
- **The commit's files, nothing else:** the checkout's `.git` is removed,
  and links are written as files holding their targets
  (`core.symlinks=false`), so nothing in a checkout reads from outside it.

## Identity, and versions

A package is still a directory (P1 of step 26): a checkout's is a function
of the URL and the rev, so one commit is one package however many
packages depend on it, and must have one name. A program and a library it
uses can both depend on `hashmap` at one rev, and `hashmap::Map` is one
type.

Two revs of one repository would be two packages, with two types called
`hashmap::Map`. That's sound, as long as each has a name of its own, but
it's rarely what was meant, so for now it's an error:

```text
error: two versions of `https://github.com/lkurcak/t-hashmap`: `5f0c1a2`
       in `t.toml`, and `9e8d7c6` here; a program has one version of each
       package, for now
```

The URL is compared as written, less a trailing `/`: two spellings of one
repository are two packages, which the one-name rule then catches.

## Paths inside a checkout

A package from git can have `path` dependencies, as any package can, but
only on packages in its own repository:

```text
error: `../outside` is outside the repository of `<g@0b13286>`: a package
       from git depends by `path` only on packages in its own repository
```

Outside the checkout is wherever the cache happens to be, which no
package's author chose; inside it is a part of the commit. A package from
git can depend on others from git, which are fetched in turn.

## Why it's sound

- **P1, identity.** As in step 26: a checkout's directory is a function of
  (URL, rev), each directory is read once, and a package has one name.
- **P1′, content.** What's read for a `rev` is the files of the commit
  with that hash: checked after checkout, with nothing added (no `.git`,
  no links out), and with line endings as committed (`core.autocrlf` is
  off). So a package from git checks the same
  on every machine, as P2 of step 26 needs. The hash is SHA-1 for most
  repositories, which is weaker than it was; a SHA-256 repository's
  64-digit hashes are accepted too.
- **P3, reading ends.** A commit can't contain its own hash, so a package
  from git that depended on its own repository would name another rev of
  it, which is the two-versions error; and every other cycle is one of
  directories, as in step 26.
- **P4, nothing new to trust.** Fetching runs no T code and no command
  from the repository; what crosses into the program is what step 26 lets
  cross.

## Design decisions worth noticing

- **`rev` only, no lockfile.** A full hash already pins everything, so a
  lockfile would repeat the manifest. A `t.lock` and `t update` come with
  the first source that can move: a branch, a tag, or versions.
- **The `git` command, not a git library.** It's one less large
  dependency, and the one thing that already knows the user's
  credentials. A machine with git dependencies needs git installed.
- **Fetched on demand, everywhere.** The editor fetches too, as an IDE
  does through its build tool: a one-time wait per commit, and nothing
  after.
- **`t-lang`, not `t`, for the cache.** A one-letter directory in a shared
  place is a clash waiting to happen.

## Deliberately missing (coming later)

- `branch` and `tag`, with a lockfile and `t update`; versions and a
  registry.
- A package in a repository's subdirectory (a `dir` key).
- Two versions of one package in a program.
- Cleaning the cache; fetching without checking (`t fetch`).
- Submodules: a checkout is the commit's own files. (A filter the user's
  own git configuration sets up, such as Git LFS's, still applies.)

## Exercises

1. Two `t`s fetch the same commit at once, into one cache. Walk through
   `git::fetch` and show that each ends with the full checkout, and that
   neither can see a half-written one. What would go wrong if it checked
   out straight into the final directory?
2. The URL is compared as written. Make `https://github.com/a/b` and
   `https://github.com/a/b.git` the same package. What could go wrong for
   a `file://` URL, where `b` and `b.git` can be different directories?
3. Why can't a package from git depend on `path = "../sibling"`, when the
   same package checked out by hand beside its sibling could? What would
   it take to allow it soundly?
4. Allow two versions of one package in a program, each with its own
   name. What does the chain rule of step 26 say about them, and is that
   the right answer?
5. A SHA-1 collision would make one `rev` two sets of files. Which of the
   lemmas above would fail, and what would a program notice?

## Reading

- Cargo's documentation, *Specifying dependencies from git repositories*:
  the design this follows, and where it goes further (branches, tags, a
  lockfile).
- Cox, *Minimal Version Selection* (2018): versions as what comes after
  pinned commits.
- Torres-Arias et al., *On Omitting
  Commits and Committing Omissions: Preventing Git Metadata Tampering That
  (Re)introduces Software Vulnerabilities* (2016): what a branch can be
  made to mean, and why a full hash can't.
- Stevens, Bursztein, Karpman, Albertini and Markov, *The First Collision
  for Full SHA-1* (2017): why git is moving to SHA-256.
