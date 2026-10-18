# Changelog

## 0.6.0 (2026-10-18)

Step 6a.1: linear types. See
[docs/06a.1-linear-types.md](docs/06a.1-linear-types.md).

### Added

- A built-in trait `Discard`: values of its types may be thrown away.
  `#[derive(Discard)]` and `impl Discard for P {}` say a struct or enum is,
  and need every field to be; a derived impl needs of the type parameters
  only what the fields do. `int`, `bool`, `()`, `!`, functions and
  references always are, and `Box<T>` is when `T` is.
- `replace(&mut x, v)`, built in: puts `v` in `x` and gives back what was
  there, for any type.
- `swap(&mut a, &mut b)`, built in: exchanges two values, for any type.

### Changed

- Values are linear: one whose type isn't `Discard` must be used up, moved
  or taken apart by a pattern, on every path that goes on. Letting it go
  out of scope, ignoring it as a result, binding it to `_`, or giving its
  place a new value (`x = v`, `*r = v`) is an error that says how to fix
  it. Paths that never go on, after a call that returns `!`, are exempt.
- Matching a value that isn't `Discard` by value against a struct or
  variant pattern uses it up, in every arm; the parts the pattern doesn't
  bind must be `Discard`.
- `trait Copy: Discard`: `#[derive(Copy)]` needs `#[derive(Discard)]` too,
  and the error's fix adds it.
- A `T` isn't `Discard` unless a bound says so: generic code that drops a
  `T` needs `T: Discard`, or `T: Copy`.
- An unused variable whose type isn't `Discard` is warned about with "use
  it up" rather than a fix to rename it `_x`, which wouldn't help.
- Every struct and enum in the tests, docs and examples is
  `#[derive(Discard)]` now.
- The interpreter moves every value whose type isn't `Copy`, not only
  those holding a box, and stops with "type soundness violated" where a
  value that isn't `Discard` is thrown away, but on the way out of a panic.

A soundness pass over traits, linear types, the call rule and `swap`, each
attempt a test (`tests/t/*soundness_*.t`).

### Fixed

- A borrow passed to a call worked out what it borrows from (`make()` in
  `&*make()`) after the other arguments in the interpreter, but in order in
  the borrow checker, which so checked a different program: one where an
  argument after it could move what it then used.
- A coerced `&mut x` argument, used as a `&T`, was related to its
  parameter's type part by part, skipping the parts that didn't line up,
  so a reference out of it could outlive what it pointed to. Coerced
  arguments are now borrows taken after the arguments, as `lift` writes
  them (`&**r`), and relating two types that aren't the same up to regions
  is an internal error.
- `==` on a type with an impl of `Eq` of its own compared field by field,
  unless something else called that `eq` by name. And `==` was invisible
  to the polymorphic recursion check, so an `eq` comparing at a bigger type
  each time made monomorphisation go on forever; it now sees `==` as the
  call of `eq` it is, through references, boxes and derived impls.
- A derived impl's bounds came out as the derived trait even where a
  field's impl needed another (`Q<T>: Eq` needing `T: Show`), so a
  `#[derive(Discard)]` type could be thrown away holding a value that
  isn't, and a derived `Eq` crashed monomorphisation.
- "moved here, in an earlier time round the loop" was said of a move
  written after a borrow that waits for the arguments, with no loop.

Step 6a: traits. See [docs/06-traits.md](docs/06-traits.md).

### Added

- `trait` declarations, with methods, default methods and supertraits
  (`trait Ord: Eq`), and `impl Trait for Type`, generic ones too
  (`impl<T: Show> Show for Option<T>`), for any type.
- Bounds on type parameters of functions, structs, enums and impls:
  `T: Show`, `T: Show + Eq + 'a`. Every use must show they hold; none are
  implied.
- Inherent impls, `impl Point { .. }`, for structs and enums, with methods
  that take `self`, `mut self`, `&self` or `&mut self`, and ones that take
  none. `Self` is the impl's type in it.
- Method calls `x.m(..)`, and `x.m::<int>(..)` for a method's own type
  parameters. They look through references and boxes to what `x` is, and
  take `&` or `&mut` of that if the method's receiver wants it. Inherent
  methods come before traits'; two traits with a method of the name are
  ambiguous.
- Paths to methods: `Point::new(1, 2)`, `Show::show(&x)`, `T::m(..)`, and
  `Point::len` without `(..)` as a function value.
- A call takes its borrows of places (`&v`, `&mut v`, and a method
  receiver's) after its other arguments, so `v.push(v.len())` and
  `f(&mut v, v.len())` work, and so does `v.push({ v.clear(); 1 })`.
- `Eq` is a trait, behind `==` and `!=`, and `#[derive(Eq)]` compares field
  by field. An impl of `Eq` can compare as it likes, and a derived one uses
  it for that field.
- `Copy` is a trait; `#[derive(Copy)]` is an impl of it, and so is
  `impl Copy for P {}`. `T: Copy` lets generic code copy a `T`. A derived
  impl needs of the type parameters only what the fields do.
- Coherence: no two impls of a trait for types that could be the same. And
  resolution always ends: impls need things of smaller types, or, for
  blanket impls like `impl<T: A> B for T`, of other traits with no way back.
- A trait impl holds at every lifetime: each lifetime and type parameter in
  its type appears once, with no `'static` and no lifetime bounds, since
  resolution doesn't look at lifetimes.
- The polymorphic recursion check follows calls of traits' methods to the
  impls they may run.
- Lint warnings for methods: `dead_code` for inherent methods nothing uses,
  and the naming lints for traits and methods. No `unused_variables` for an
  unused `self`.
- `t lsp`: hover, go to definition, references and rename for traits and
  methods (renaming a trait's method renames its impls'), and traits and
  impls with their methods in the outline.
- The tree-sitter grammar knows traits, impls, receivers and method calls.

### Changed

- `==` on a struct or enum needs it to implement `Eq`: add
  `#[derive(Eq)]`. `==` on a `T` needs a bound `T: Eq`.
- `s.f(x)` is a call of method `f`. To call a function held in field `f`,
  write `(s.f)(x)`; the error says so.
- `trait`, `impl` and `for` are keywords, and `self` can only be a method's
  receiver.

## 0.5.1 (2026-10-17)

Step 5.1: control flow, and the type with no values. See
[docs/05.1-control-flow.md](docs/05.1-control-flow.md).

### Added

- `return e`, and `return` for `return ()`, from a function or closure. A
  closure's result type is worked out from its `return`s as well as its
  body.
- `loop { .. }`, which runs until a `break`. `break e` gives it the value
  of `e`, and `break` gives it `()`; with no `break` that gets out, it never
  ends.
- `break` and `continue`, in a `loop` or `while`. A `while` loop's `break`
  can't have a value, since the loop can also end without one. Neither can
  get out of a closure.
- `!`, the type with no values, said *never*. `return`, `break` and
  `continue` are of type `!`, as is a `loop` that never ends and a `match`
  with no arms, and a `!` can be used as any type, as it never gives a
  value: `None => return -1` fits as an arm of an `int` `match`. It can be
  written, as in `fn forever() -> ! { loop {} }`, and anywhere else a type
  can: an `Option<!>` is never `Some`, so a `match` needn't cover it.
- A block that never gets to its end, say as it ends with a `return`, is a
  `!`, so a function can end with `return x;`.
- A warning, `unreachable_code`, for code after something that never
  finishes, like a `return`, as rustc gives.
- The borrow checker follows jumps: a `return`, `break` or `continue` ends
  the scopes it leaves, so a box moved just before a `break` is fine. Dead
  code isn't borrow checked.
- The interpreter frees what a jump leaves, the parts of an expression it
  had worked out included, as in `f(Box::new(1), return 2)`.
- Labels on loops: `'outer: loop { .. }` and `'outer: while c { .. }`,
  and `break 'outer`, `break 'outer x` and `continue 'outer` to leave or
  go round a loop around the innermost. A loop can't have the label of
  one around it. See
  [docs/05.1-control-flow.md](docs/05.1-control-flow.md#labels).
- A warning, `unused_labels`, for a label no `break` or `continue` names.
  Its fix removes it.
- Syntax highlighting for all of the above.

### Changed

- `return`, `break`, `continue` and `loop` are keywords.
- `let x = match n {};` makes `x` a `!`, rather than being an error for
  not saying what type it is.

## 0.5.0 (2026-10-16)

Step 5: ownership, borrowing and pointers. See
[docs/05-ownership.md](docs/05-ownership.md).

### Added

- References: `&x` and `&mut x`, of types `&T` and `&mut T`, and `*r` for
  what `r` points to. A field of a reference is a field of what it points
  to (`r.x`), and a `&mut T` can be used as a `&T`, and a `&Box<T>` or
  `&&T` as a `&T` (deref coercion).
- Lifetimes: `&'a T`, `fn f<'a>(..)`, `struct S<'a>`, bounds like
  `'b: 'a` and `T: 'a`, and Rust's lifetime elision for signatures. A
  struct or enum can hold a reference, so a type can hold itself through
  one. A lifetime left out in a function type is one each call picks:
  `fn(&int) -> int` takes references of any lifetime.
- `Box<T>`, a value on the heap: `Box::new(x)` allocates, `*b` is what it
  holds, and it's freed when its owner goes away. A type can hold itself
  through a box: `enum List { Nil, Cons(int, Box<List>) }`.
- Mutation: `let mut`, `mut` parameters and pattern bindings, assignment to
  variables, fields and `*r`, and `+=`, `-=`, `*=`, `/=` and `%=`.
- `while` loops.
- A reference or box to something with no values, like `&Never`, has
  none either, and nor does a type that must hold itself forever, like
  `struct Stream { next: Box<Stream> }`: a `match` needn't cover them.
  Rust asks for those arms, since unsafe code there may make a `&Never`.
- A warning, `infinite_types`, for a struct or enum that holds itself with
  no way to stop, like `struct Stream { next: Box<Stream> }`, or two that
  hold each other: it has no values.
- `let x;`: a variable declared with no value, given one later by
  assignment. It can't be used before then, and it can be assigned once
  without being `mut`.
- Moves: using a value whose type isn't `Copy` moves it, and it can't be
  used again until it gets a new value. `int`, `bool`, `()`, functions and
  shared references are `Copy`, and so are structs and enums with
  `#[derive(Copy)]`, if their fields are. `#[derive(..)]` is the second
  attribute; `Copy` is all it derives.
- The borrow checker (`src/borrowck`), as in rustc since 2018: non-lexical
  lifetimes, worked out on a lowered form of each function (MIR). It
  rejects using a value after it's moved, moving out from behind a
  reference, changing what isn't `mut` or is behind a `&`, using or
  changing something in a way that clashes with a borrow of it, references
  that outlive what they point to, and functions that need more of their
  lifetimes or type parameters than their signatures say.
- Matching on a reference looks through it, and binds variables by
  reference (match ergonomics).
- A warning, `unused_mut`, for a variable declared `mut` that's never
  changed. Its fix removes the `mut`.
- The interpreter keeps memory the way a compiled program would, and
  checks for dangling references, uses after moves, double frees and leaks
  as it runs.
- Errors can have notes, pointing at other places that explain them:
  "`x` is borrowed here".
- Syntax highlighting for all of the above.

### Changed

- `Box` is a built-in type, so a struct, enum or type parameter can't be
  called that.
- Closures can only capture values that are `Copy` and hold no
  references, since what they capture is copied into them, and can't
  change what they capture. Assigning a closure that captures, or a
  function read through a reference, is an error, as returning and storing
  one are.
- A struct or enum that contains itself suggests holding itself through a
  pointer, like `Box<List>`, instead.

## 0.4.5 (2026-10-15)

### Added

- `t --help` (or `-h`) prints what each command does, on stdout, and `t
  --version` (or `-V`) prints the version. Both used to print the usage as
  if they were a mistake, and exit with 2.

## 0.4.4 (2026-10-15)

### Added

- `#[allow(..)]` turns lints off: before a `fn`, `struct`, `enum`, field or
  variant, top-level or nested, it turns off the lints it names in all of
  it, as in Rust. It goes before or after the doc comment; `t fmt` puts it
  after. A name that isn't a lint's is a new warning, `unknown_lints`, with
  a fix when it's close to one (`dead_cod` → `dead_code`). `#[allow]` is the
  only attribute; any other is a syntax error, as is one before anything
  else, like a `let`.
- `t check` and `t run` name a lint the first time it warns, with how to
  turn it off: "= note: `#[allow(dead_code)]` turns this off". `t lsp`
  already gave the lint as each warning's code.
- Syntax highlighting for attributes.

### Changed

- Syntax highlighting: the `x` of a shorthand `P { x }`, in a struct
  literal or pattern, is highlighted as the variable it is rather than as
  a field, as `t lsp` takes it for hover and rename.

## 0.4.3 (2026-10-15)

### Added

- Naming lints, as in Rust: `non_camel_case_types` for a struct, enum,
  variant or type parameter whose name isn't in `UpperCamelCase`, and
  `non_snake_case` for a function, parameter, variable or field whose name
  isn't in `snake_case`. Leading underscores don't count and digits are
  fine (`_unused`, `Vec2`, `x1`), and an acronym is one word, so
  `HTTPServer` should be `HttpServer`, and `IO` should be `Io`: stricter
  than rustc, as Rust's style guide asks, so all capitals are left for
  globals. Each is reported once, where the name is declared, as rustc
  words it: "type `my_point` should have an upper camel case name:
  `MyPoint`". The fix renames it everywhere, as `t lsp`'s rename does, so
  it's offered only when the program checks and the new name isn't a
  keyword and clashes with nothing. The test programs that broke the rules
  are renamed.

- Nested functions: a `fn` can be declared as a statement in any block. It
  means exactly what it would at the top level; only its name is scoped, to
  the whole block, so it can be used before it's declared, be recursive, and
  call the other functions of its block. It can be generic and have a doc
  comment, and it shadows any outer function of the same name. It can't
  capture: using a local of a function it's in is an error, "can't capture
  `x` in a nested `fn`; use a closure", whose fix makes it a closure when
  that works. See docs/03-inference.md.
- Local types: a `struct` or `enum` can be declared in a block too, by the
  same rules. Its name is in scope in the whole block and shadows any outer
  type of the same name, and its values can leave the block by inference.
  It can't use the type parameters of the function it's in. When two
  different types of the same name meet in a type error, the message says
  so, and where each is declared.
- The checker resolves each use of a function or type to its definition,
  through the scopes it's in, rather than by name. Once checked, nested
  functions and local types are moved to the top level under names of
  their own (`main::helper`, `main::Point`), which `t mono` shows, so
  monomorphisation and the interpreter never see one. Messages, printed
  values, `t lsp`'s hover and go to definition show and find them as
  written, even when several share a name.
- "Never used" warnings for nested functions that nothing in their block
  uses, and the dead-code warnings for structs, enums and variants cover
  local ones.
- `t lsp` finds references, highlights every place something is named
  (its declaration as written, its uses as read), and renames: variables
  and parameters (of closures and patterns too, every alternative of an
  or-pattern), functions, structs, enums, variants, fields and type
  parameters, top-level or local. All of these follow what each name
  refers to, not how it's spelled, through the same query
  (`Info::occurrences`). Renaming either side of a shorthand `P { x }`
  spells out the other. A rename is refused if the new name isn't an
  identifier, is a keyword, or is `int` or `bool` for a type, and if it
  would change what any name refers to: the renamed program is checked
  again, and the message says what it would clash with. Built-ins and
  `main` can't be renamed, and a file that doesn't check can't be renamed
  in.
- `t lsp` goes to type definitions, from a variable, parameter, field or
  function to each struct and enum in its type, and gives an outline of
  the file (document symbols): functions, structs and enums, with fields
  and variants under their type and nested functions and local types
  under their function, each with its signature. The outline comes from
  the parse alone, so it's there while the file has a type error.
- In a shorthand `P { x }`, hover, go to definition and the rest take
  `x` to be the variable rather than the field, as renaming does. The use
  of a function with type arguments, like `id::<int>`, is just `id` to
  them.
- `t fmt` formats nested functions and local types like top-level ones,
  with their doc comments, and keeps blank lines around them as around any
  statement. The tree-sitter grammar parses them.

## 0.4.2 (2026-10-15)

### Added

- Doc comments: `///` before a `fn`, `struct`, `enum`, struct field or enum
  variant documents it, in Markdown. `t lsp` shows it on hover, after the
  signature, on the declaration and on every use. Doc comments are part of
  the AST, so `t fmt` prints them from there and its check that the output
  is the same program covers them. `////` is a plain comment, as in Rust.
- A warning, `unused_doc_comments`, for a `///` that documents nothing, such
  as one before a statement. Its fix makes it a plain `//` comment.
- The tree-sitter grammar tells doc comments from plain comments, and the
  highlight queries capture them as `@comment.documentation`.
- `cargo test` checks the T code in the README and `docs/`: every
  ```` ```rust ```` block must parse and be formatted, and a whole program
  must run and print what its `// expect:` comments say, as in `tests/t`.
  Fragments and deliberate errors are marked in the fence
  (```` ```rust,fragment ````, ```` ```rust,error ````,
  ```` ```rust,unformatted ````); see the README's Tests section.

### Fixed

- Docs whose code no longer worked: step 3 opened with an `adder` that
  returns a closure capturing `n`, an error since 0.4.0, and step 4's
  opening program left a variable unused. Several examples weren't
  formatted, and some mixed items with statements, which doesn't parse.

## 0.4.1 (2026-10-15)

### Added

- `t fmt <file>`, a formatter with one style and no options: 100 columns,
  four-space indentation, a line broken only where it doesn't fit (with
  Wadler's "prettier printer" algorithm). It keeps comments, and single
  blank lines between items, statements and arms, and lines up comments at
  the ends of consecutive lines. `t fmt --check` prints a diff and fails if
  a file isn't formatted. It parses its own output before writing, and
  refuses if that's a different program. See [docs/fmt.md](docs/fmt.md).
- `t lsp` formats documents, so editors can format on request or on save.
  See the README for Neovim and Helix; the Helix config formats on save.

### Changed

- `t mono` prints in `t fmt`'s style: short declarations and blocks that
  are one expression go on one line, and long lines break.
- Building T needs Rust 1.99 or later, up from 1.88. T is an application, so
  it tracks the Rust it's built with rather than holding to an old one.

### Fixed

- `t mono` printed a range pattern like `0..1` as `0`, and `(a < b) < c`
  without its parentheses, which doesn't parse.

## 0.4.0 (2026-10-15)

Step 4: generics. See [docs/04-generics.md](docs/04-generics.md).

### Added

- Generic functions, structs and enums: `fn id<T>(x: T) -> T`,
  `struct Pair<A, B> { .. }`, `enum Option<T> { Some(T), None }`, and
  types like `Option<int>`. Type arguments are inferred at each use, or
  written out with a turbofish: `id::<int>(1)`, `Option::<int>::None`, with
  `_` for parts to infer (`second::<bool, _>(..)`). Inside
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
