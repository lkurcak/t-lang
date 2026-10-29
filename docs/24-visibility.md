# Step 24: visibility and effect inference

Code: `Vis` and `Level` in `src/ast.rs`, `Modules::visible` and
`Resolver::exports_reachable` in `src/check/modules.rs` (who can see
what), and `src/check/infer.rs` (effect inference), with the
`Deferred` checks of `src/check/effects.rs`.

Until now `pub` did two jobs. Inside a program it let one module use
another's items. For a library it was a promise to other packages: what
they may count on, which changing breaks them. The first is cheap, and
wanted all the time; the second is a contract. So there are three levels
now, one for each boundary the checker treats differently:

- no keyword: seen in its module, and the modules in it, as before;
- `pub`: seen anywhere in its package;
- `export`: seen by other packages too. It's the package's interface.

And what only a package's own code can count on needn't be written down:
a function that isn't exported, and doesn't say `with`, has the effects
its body has.

```rust
mod parse {
    // Seen in the package, not outside it. Its effects are its body's:
    // `+` may panic.
    pub fn digits(s: &[u8]) -> i64 {
        let mut n = 0;
        for c in s.iter() {
            n = n * 10 + (*c - b'0') as i64;
        }
        n
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&parse::digits(b"42"))?; // expect: 42
    Result::Ok(())
}
```

Nothing there says `with panic`, though `digits` and `main` may panic.

## Who sees what

An item is seen from module `m` if it's private and `m` is in its module,
or it's `pub` and `m` is in its package, or it's `export`
(`Modules::visible`). Fields and inherent methods say for themselves, and
are seen no further than their type: `export` on a field of a struct that
isn't exported is an error.

A library's `pub` items are its own. `std::rt`, which reports the error
`main` gives back, is how the runtime does that, not something for a
program:

```rust,error
fn main(os: std::Os) {
    std::rt::report(&1i64, os.stderr);
}
// error: module `rt` isn't exported by `std`
```

The rules that were about `pub` sort themselves into the two:

- **Abstraction** stays per module (lemma M2 of docs/06d-modules.md): a
  struct with a field a module can't see can't be made or taken apart
  there. An exported struct with a field that's only `pub` is abstract
  outside its package, by the same rule.
- **What a signature names** must be seen as far as the item is: a `pub`
  function's types at least `pub`, an exported one's exported.
- **Derive bounds** are written only by an exported struct with fields
  that aren't (M5 moved to the package's boundary). Inside the package,
  what's worked out from the fields is used: code there that counts on
  it is checked again when the fields change.
- **`unused_effects`** spares only exported functions, which may say an
  effect to keep the right to have it later. A `pub` one's callers are
  in its package.
- **A field's refinement** names only fields seen exactly as far as it:
  whoever can write one must keep the invariants that name it.
- **Dead code**: what's exported is used by other packages; a `pub`
  function nothing in its package uses is dead.

A program has no other packages using it, but `export` means the same in
it, checked as if it had: so the rules can be tried in a program, and a
program can become a library without changing what it says.

### What's exported can be named

```rust,error
mod m {
    export fn f() -> i64 { 1 }
}

fn main() {
    let _x = m::f();
}
// error: `f` is exported, but other packages can't name it: module `m` isn't exported, and no `export use` names it
```

Every exported item must be reachable from its package's root through
exported modules and `export use`s alone (`exports_reachable`), so the
interface is what the root says, read by following `export`s down, and
nothing promised is hidden where only the package's own modules see it.
`export use` can only re-export what's exported already, as `pub use`
couldn't make public what isn't.

## Effects of what isn't exported

A function, inherent method or nested function that isn't exported and
says no `with` has its body's effects. `main` too: nothing calls it but
the runtime, which takes any. What's written stays as it was: an
exported function's effects, a trait's methods' (and so its impls'),
bounds and effect parameters. A `with` written anywhere is a promise the
body is checked against; `with {}` says there are none.

```rust,error
// An exported function says its effects: what it calls may have no more.
fn h() -> i64 { panic("no") }

fn g() -> i64 { h() }

export fn f() -> i64 { g() }

fn main() {
    let _ = f();
}
// error: `g` may panic, but `f` doesn't say `with panic`
```

The error says why, call by call, as it's written nowhere: notes that
`g` may panic at its call of `h`, and `h` at its call of `panic`.

`unsafe` is never inferred. Saying it says the callers keep obligations
the checker can't check, which is a promise only a signature can make: a
body can't make it for its function.

### How

Bodies are checked callees first (`body_groups`). Which may call which is
judged from the names in each body, every function and method it
mentions, which is more than it calls: a group is the functions that may
call each other, a strongly connected component of that graph (Tarjan's
algorithm finds them callees first). So a call of a function in an
earlier group has its effects as if they were written.

Until its group is done, a function's effects are a placeholder,
`Type::Inferred(f, args)`: what will have been inferred for `f`, at the
type arguments of the call. Within a group, a call of another of it has
one, and so does whatever its effects flow into: a closure that calls it,
an effect parameter decided by it, its type as a value. The checks of
those (`Within`s: a closure meeting a bound, a function used as a pointer
type, two sets that must be equal) wait until the group is done
(`Deferred`). Then each function's effects are the least that holds for
every site of its body (`solve_group`): start with none, add each site's,
with the group's placeholders as they stand, and go round until nothing
changes. Then the waiting checks are made, and each function's type has
its effects, for the groups after.

## Why it's sound

**E1. What's inferred is enough.** At the solution, each inferred
function's effects have every effect of every site in its body. That's
all step 7's lemma (docs/07-effects.md) used of a declared set: so a
function that has no `panic`, said or inferred, reaches none. The
interpreter's oracle checks it, with the inferred effects in each copy's
signature.

**E2. A least solution exists, and is found.** Each step only adds
effects, and a function's are drawn from finitely many: `alloc`, `panic`,
`unsafe`, its own effect parameters and the associated effects of its
parameters, which a call at other arguments maps back into its caller's
by substitution. Union is monotone, so going round from nothing reaches
the least fixed point (Kleene, Tarski). Polymorphic recursion is ruled
out (docs/04-generics.md), so the arguments a group calls itself at are
finitely many too.

**E3. What waited is decided right.** A check that waited is an upper
bound, `sub ⊆ sup`, or two of them, for an equality. Its `sub` only grows
with the solution, so if it holds at any solution it holds at the least:
checking it there accepts exactly what some solution would.

**E4. Packages are still checked apart** (docs/14-packages.md). Nothing
inferred crosses a package: what isn't exported can't be named outside,
and everything about an exported item that a dependent sees is written.
A library's inference is done before any program is checked.

**E5. Changing a body can't quietly change what other packages count
on**: what they count on is written. Inside the package, a changed body
changes its callers' effects, and they're all checked again with it.

## Design decisions worth noticing

- **Two boundaries, three levels.** There's no `pub(super)` or `pub(in
  path)`: the checker treats the module and the package differently, and
  nothing else.
- **The short keyword for the cheap level.** `pub` is what a program
  writes all the time. Forgetting `export` fails loudly, in the dependent;
  an accidental `pub` promises nothing.
- **Inference only where callers are checked together**: not across a
  package, and not for what a trait's method says, which every impl must
  keep. An impl's method still says its effects, or has none.
- **Callees first, rather than all at once.** Without recursion, a call
  of an inferred function is checked exactly as if its effects were
  written, with the same errors, where they're found. Only a recursive
  group waits.
- **Errors explain the chain.** Inference's real cost is that an error
  can be far from its cause: the notes walk from the exported function to
  the operator.

## Deliberately missing (coming later)

- Type aliases, and what `pub` and `export` mean for them: the next step.
- Semver checks of an interface against an earlier version.
- `missing_docs` for exported items.
- An impl of a trait whose bounds need an effect still being inferred, in
  its own group, fails rather than waits (`traits::fits`): write the
  effects there.
- User-written library packages, which `export` is ready for.

## Exercises

1. Why must an exported item be reachable from its package's root?
   Write two modules that would each think the other exports something.
2. Make `body_groups` put every function in one group. What still works,
   and what do the errors look like?
3. Infer `unsafe` too, and write a program that does something unsafe
   without an `unsafe` block anywhere.
4. Why is the least solution the right one, and not some other? Find a
   recursive group with more than one solution.
5. An impl's methods say their effects. What would inferring them need,
   for an impl of an exported trait for an exported type?

## Reading

- Daan Leijen, *Koka: Programming with Row Polymorphic Effect Types*
  (2014): effects inferred for every function, by Hindley–Milner with
  rows.
- Lucassen and Gifford, *Polymorphic Effect Systems* (1988): the first
  effect inference.
- Robert Tarjan, *Depth-First Search and Linear Graph Algorithms* (1972):
  strongly connected components, in the order they're found.
- The C++20 modules proposal (P1103): `export` as a module's interface.
- The Rust reference's *Visibility and privacy*, and RFC 1422 (`pub(restricted)`),
  for what three levels leave out.
