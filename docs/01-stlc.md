# Step 1: the simply typed lambda calculus

Code: `src/check/mod.rs` (the type system) and `src/interp.rs` (what programs mean).

## What a type system actually is

A type system is a set of **rules** that classify expressions by the kind of
value they produce, *without running them*. The checker reads the program
once, applies the rules, and either accepts it or points at the place where no
rule applies.

The point is a guarantee, not just catching typos:

> **Type soundness:** if the checker accepts a program, running it never gets
> *stuck*, meaning it never reaches an operation that makes no sense, like
> `true + 1` or calling the number `5`.

That's "well-typed programs don't go wrong" (Robin Milner, 1978). Everything
else in this project builds on that sentence.

## Judgments and rules

The core statement a type system makes is a **judgment**:

```text
Γ ⊢ e : T        "in context Γ, expression e has type T"
```

Γ (gamma) is the **context**: the types of the variables in scope. In the code
it's `Checker::locals` plus `Checker::globals`.

Rules are written as fractions: premises on top, conclusion below.

```text
  Γ ⊢ l : int    Γ ⊢ r : int
  --------------------------- (T-Arith)
       Γ ⊢ l + r : int
```

"If `l` is an int and `r` is an int, then `l + r` is an int." Every arm of
`Checker::synth` is one such rule, and the rule is written above it in a
comment. Read them side by side: the code is the rules, written as a recursive
function.

The two most important rules:

```text
   (x : T) ∈ Γ                     Γ ⊢ e1 : T1    Γ, x : T1 ⊢ rest : T
  ------------- (T-Var)           ------------------------------------- (T-Let)
   Γ ⊢ x : T                         Γ ⊢ { let x = e1; rest } : T
```

T-Let is where **scoping** comes from. `rest` is checked in a *bigger*
context (`Γ, x : T1`), and nothing outside the block ever sees `x`. In code
that's the push onto `locals` and the `truncate` when the block ends.

```text
  Γ ⊢ f : fn(T1..Tn) -> R    Γ ⊢ a1 : T1  ..  Γ ⊢ an : Tn
  -------------------------------------------------------- (T-App)
                  Γ ⊢ f(a1..an) : R
```

T-App is function application, the "lambda calculus" in the name. Function
types are ordinary types, so functions can be passed around
(`tests/t/first_class_fns.t`).

## Soundness = progress + preservation

You prove soundness in two halves (TAPL chapter 8):

- **Progress:** a well-typed expression is either already a value, or it can
  take another step of evaluation. It's never stuck.
- **Preservation:** if a well-typed expression takes a step, the result has
  the *same* type.

Put together: start well-typed, stay well-typed at every step, and never get
stuck.

**Where you can see this in the code:** `src/interp.rs` calls `unsound(..)`
everywhere a value could have the wrong shape, such as the `int()` and `bool()`
helpers and calling a non-function. Those calls are exactly the "stuck" states.
Soundness says they're unreachable for checked programs. If one ever fires,
that's a bug in the type checker.

### What soundness does *not* promise

`tests/t/err_div_zero.t` and `err_overflow.t` are well-typed and still fail at
runtime. That isn't a contradiction. Soundness guarantees there are no *stuck*
states, and an explicit, defined error is not a stuck state. Every type system
draws a line between what it rules out statically and what it leaves to
runtime checks. Moving that line is what fancier type systems are for. For
example, refinement types can rule out division by zero.

## Design decisions worth noticing

- **Syntax vs. semantics of types.** `ast::TypeExpr` is a type *as written*;
  `types::Type` is a type *as understood*. `resolve` converts between them and
  rejects `string`. Keeping them apart matters later, when `T` in a generic
  function and `T` in the source are different things.
- **No truthiness.** `if 1 { .. }` is a type error, unlike C.
  `bool` and `int` are different types.
- **`if` without `else` has type `()`.** Otherwise, what would it produce
  when the condition is false?
- **No `==` on functions.** Deciding whether two functions are equal is
  undecidable in general, so T-Eq excludes function types.
- **Syntax-directed checking.** Every expression form has exactly one rule,
  so the rules *are* the algorithm. Even in STLC we got lucky:
  `let x = e` doesn't need an annotation only because `e`'s type can always
  be computed bottom-up. As soon as you want `let id = fn(x) { x }` with no
  annotation on `x`, you need **inference** (step 3).
- **Two passes over functions.** All signatures are collected before any body
  is checked. That's why recursion and calling functions defined later both
  work. A function's own name is in Γ while its body is checked.

## Deliberately missing (coming later)

| Missing | Why it's missing | Step |
|---|---|---|
| structs, enums, `match` | algebraic data types | 2 |
| `fn(x) { x }` without annotations | type inference | 3 |
| closures (functions that capture locals) | capturing an environment needs memory for it, which runs straight into "no GC" | 3 or 7 |
| `fn id<T>(x: T) -> T` | parametric polymorphism | 4 |
| mutation, loops | effects and ownership; recursion is enough for now | later |

## Exercises

1. Add a `print_bool : fn(bool) -> ()` builtin. Touch `BUILTINS`,
   `builtin_type` and `Interp::call`.
2. Comment out the `self.check(c, &Type::Bool)?` in the `If` case, then write a
   program that makes the interpreter hit `unsound`. You've just broken
   soundness on purpose.
3. Why does T-Eq need `check(r, &t)` and not just `synth` on both sides? What
   would go wrong if `1 == true` were allowed?

## Reading

- *Types and Programming Languages* (Pierce), chapters 8–11.
- *Crafting Interpreters* (Nystrom), chapters 4–8, for the lexer and parser.
