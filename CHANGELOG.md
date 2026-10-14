# Changelog

## Unreleased

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
