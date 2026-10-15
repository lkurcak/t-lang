# Changelog

## Unreleased

Step 4: generics. See [docs/04-generics.md](docs/04-generics.md).

### Added

- Generic functions, structs and enums: `fn id<T>(x: T) -> T`,
  `struct Pair<A, B> { .. }`, `enum Option<T> { Some(T), None }`, and
  types like `Option<int>`. Type arguments are inferred at each use, or
  written out with a turbofish: `id::<int>(1)`, `Option::<int>::None`. Inside
  a generic body a type parameter is opaque, and `==` on one is an error
  until traits. Closures and `let`s stay monomorphic.
- Monomorphisation, as a pass between checking and running: each generic
  function and type is copied once per list of type arguments it's used
  with. `t mono <file>` prints the result. Polymorphic recursion that would
  need infinitely many copies is an error. Each copy is checked again, so
  using a generic that returns or stores its argument at a function type
  (`id(|x| x + n)`) is rejected like the non-generic version would be.
- `t lsp` answers hover, go to definition and inlay hints (inferred types,
  and the type arguments of generic uses).
- A tree-sitter grammar with highlight queries, in `editors/tree-sitter-t`,
  and setup notes for syntax highlighting in Helix and Neovim.

### Changed

- Closures that capture variables are downward-only: they can be called and
  passed to functions, but not returned or stored in a struct or enum, since
  the variables they capture live on the stack. `fn adder(n: int) ->
  fn(int) -> int { |x| x + n }` is now an error, and so is returning a
  function-typed parameter. Closures that capture nothing, and top-level
  functions, go anywhere as before. This keeps closures compilable without a
  heap or a garbage collector.
- The roadmap is reordered around compiling to bare metal: generics, then
  ownership (with types that hold themselves through pointers, in place of
  a separate recursive-types step), traits, effects, and refinement types.

## 0.3.0 (2026-10-14)

Step 3: type inference and closures. See
[docs/03-inference.md](docs/03-inference.md).

### Added

- Closures: `|x| x + 1`, `|x: int, y| -> int { x * y }`, and `|| 1` with no
  parameters. They can use the locals around them, be passed, returned and
  stored in structs, and have the same `fn(..) -> ..` types as top-level
  functions.
- Type inference inside function bodies. A closure parameter's type comes
  from its annotation, from the type expected where the closure is written
  (`apply(|p| p.x, ..)`), or else from how it's used, by unification. A
  variable whose type is never worked out is an error asking for an
  annotation, as is one that would need an infinite type (`|f| f(f)`).
- `let x = match n {};` with no arms and no annotation takes the type `x` is
  used at, instead of being an error.

## 0.2.3 (2026-10-14)

### Added

- A warning for code that can never run (`dead_code`): functions `main`
  doesn't reach, even if they call each other, structs `main` never builds,
  enums it never mentions, and variants it never constructs. Names starting
  with `_` are exempt. Editors fade it out.

## 0.2.2 (2026-10-14)

### Added

- Warnings. Unlike errors they don't stop the program; `t` prints them to
  stderr and runs it anyway.
- Fixes for warnings and some errors, shown as `help:` lines and offered by
  `t lsp` as quick fixes (code actions): remove an unreachable arm or
  alternative, narrow or merge a range, add the missing `;` after an `if` or
  `match` statement (or wrap it in parentheses), and use `|` instead of `||`
  between pattern alternatives.
- `t lsp` reports every warning, not just the first problem. Each warning has
  a code naming its kind (`unreachable_patterns`, `overlapping_ranges`,
  `mergeable_ranges`), and unreachable patterns are tagged as unnecessary, so
  editors fade them out.
- A warning for variables nothing reads (`unused_variables`), with a fix
  that renames `x` to `_x`, everywhere an or-pattern binds it, and
  `P { x }` to `P { x: _x }`. Names starting with `_` are exempt.

### Changed

- Unreachable arms and alternatives, ranges that can be narrowed, and ranges
  that can be merged are warnings instead of errors, and every one is
  reported instead of just the first.

## 0.2.1 (2026-10-14)

### Added

- Range patterns on ints: `1..10` (excluding 10), `1..=9`, `10..`, `..0` and
  `..=-1`. Exhaustiveness checking understands them, so
  `match n { ..0 => a, 0.. => b }` needs no `_`, and an empty range like
  `5..5` is an error.
- An error for a range whose end the patterns before it already match, with
  the narrower range to write instead: after `0..=5`, the arm `5..=10` should
  be `6..=10`. A range that surrounds an earlier one is fine
  (`5..10 => a, 0..20 => b`).
- An error for or-pattern alternatives that touch end to end, like
  `0..10 | 10..20`, suggesting the one range `0..20`. `0 | 1 | 2` is fine.

### Changed

- A non-exhaustive `match` on ints names the first range left out
  (``non-exhaustive `match`: `..=-1` not covered``) instead of `_`.

## 0.2.0 (2026-10-14)

Step 2: algebraic data types. See [docs/02-adts.md](docs/02-adts.md).

### Added

- `struct` and `enum` declarations, struct literals (with `Point { x, y }`
  shorthand), field access `p.x`, and variant constructors `Shape::Rect(2, 3)`.
- `match` with nested patterns: `_`, variables, int/bool/`()` literals,
  variants, struct patterns with `..`, and or-patterns `a | b`.
- Patterns in `let`, which must be irrefutable: `let Point { x, y } = p;`.
- Exhaustiveness checking with a counterexample
  (``non-exhaustive `match`: `Shape::Rect(_, _)` not covered``), and errors
  for unreachable arms and or-pattern alternatives.
- The empty type: `enum Never {}`, where `match n {}` needs no arms. Variants
  that hold an empty type need no arm either.
- `==` and `!=` on structs and enums that contain no functions.

### Changed

- An `if`, `match` or block at the start of a statement ends the statement,
  and without a `;` it must have type `()`, as in Rust. Before,
  `if c { 10 } else { 20 }` followed by `- x` on the next line was a
  subtraction.
- The interpreter's stack is larger, so deep recursion in debug builds hits
  the 10,000-call limit instead of overflowing the native stack.

### Not yet

- Recursive types (step 5) and generic types (step 4) are rejected.
- Range patterns are deferred.

## 0.1.0 (2026-10-13)

Step 1: the simply typed lambda calculus. See [docs/01-stlc.md](docs/01-stlc.md).

- Ints, bools, `()`, first-class top-level functions, `let`, `if`/`else` and
  recursion, with a type checker and a tree-walking interpreter.
- `print` and `print_bool` builtins.
- `t lsp`, a language server that reports errors as you type.
