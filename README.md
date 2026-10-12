# T

Like C, but a different letter.

A small systems language with no garbage collector and no runtime, built one
type-theory concept at a time, mainly to learn what those concepts are and why
they matter.

```rust
fn fact(n: int) -> int {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn twice(f: fn(int) -> int, x: int) -> int { f(f(x)) }

fn main() {
    print(fact(10));
    print(twice(fact, 3));
}
```

## Usage

```sh
cargo run -- examples/hello.t         # type-check and run
cargo run -- check examples/hello.t   # type-check only
cargo test                            # run every program in tests/t/
```

Pipeline: `lexer` → `parser` → `check` (type checker) → `interp` (tree-walking
interpreter). A real backend (emitting C) comes once the type system has
enough in it to be worth compiling.

## Tests

Each `tests/t/*.t` file is a program with its expectations in comments:

```rust
print(1 + 2);   // expect: 3
// error: expected `int`, found `bool`
```

## Roadmap

| Step | Feature | Concept | Notes |
|---|---|---|---|
| 1 ✅ | ints, bools, `let`, functions, `if` | simply typed lambda calculus, soundness | [docs/01-stlc.md](docs/01-stlc.md) |
| 2 | structs, enums, `match` | algebraic data types, exhaustiveness | |
| 3 | inference, closures | unification / bidirectional typing | |
| 4 | generics | parametric polymorphism (System F), monomorphization | |
| 5 | recursive types | iso- vs equi-recursive types | |
| 6 | traits | ad-hoc polymorphism, typeclasses, coherence | |
| 7 | ownership, borrowing | affine types, regions/lifetimes | |
| — | C backend | | when it's worth it |
