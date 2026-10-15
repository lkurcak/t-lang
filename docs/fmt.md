# `t fmt`: one style, no options

Code: `src/fmt.rs` (the command, and the check before writing),
`src/trivia.rs` (comments and blank lines), `src/pretty.rs` (AST to
layout), `src/doc.rs` (the layout engine), and `tests/fmt.rs`.

```rust
struct Point{x:int,y:int}
fn dist(p:Point,q:Point)->int{
let dx=p.x-q.x;let dy=p.y-q.y;  // no square roots yet
dx*dx+dy*dy}



fn main(){print(dist(Point{x:0,y:0},Point{x:3,y:4})); // expect: 25
}
```

`t fmt` turns that into:

```rust
struct Point { x: int, y: int }
fn dist(p: Point, q: Point) -> int {
    let dx = p.x - q.x;
    let dy = p.y - q.y; // no square roots yet
    dx * dx + dy * dy
}

fn main() {
    print(dist(Point { x: 0, y: 0 }, Point { x: 3, y: 4 })); // expect: 25
}
```

`t fmt <file>` rewrites the file; `t fmt --check <file>` rewrites nothing,
prints a diff, and fails if the file isn't formatted. `t lsp` formats the
same way when an editor asks it to. This isn't a step on the roadmap: it's
not about types. But a formatter is one of the first tools a language
needs, and building one has its own small theory, so it gets a doc like a
step.

## One style, and no options

`t fmt` has no configuration file, no flags that change the output, and it
ignores the tab size an editor asks for. Every T program is formatted the
same way, and there's no way to change that. This is Go's choice: `gofmt`
shipped with the language and has no style options, so Go code all looks
the same. Rust's `rustfmt` has dozens of options, and code from two projects
can look different.

The argument for no options:

- **Nobody's favourite, everybody's style.** What matters about a style is
  that there's one, so that any T code looks like any other and reads
  without getting used to it. The details matter much less. ("Gofmt's style
  is no one's favorite, yet gofmt is everyone's favorite", in Rob Pike's Go
  Proverbs.)
- **No arguments.** With options, every project has the discussion of which
  ones to pick, and every new contributor learns the answer. Without, the
  question doesn't exist.
- **Diffs are about code.** If everyone runs the formatter, a change never
  comes with reformatting noise, and merging is easier.
- **Options multiply the tool.** Each one doubles the layouts to test, and
  they interact, so every combination is a formatter someone has to keep
  working.

The cost is that some choices will annoy someone. T is young, so this is
the time to fix the style: before there's code, and habits, written any
other way.

The width is 100 columns. 80 comes from punched cards, and with four-space
indentation, nested blocks and descriptive names it breaks lines that read
better whole. 100 is what rustfmt uses.

## From AST to text

The printer in `src/pretty.rs` already existed, for `t mono`: it prints a
program back out from its AST. `t fmt` is that printer, plus three things
the AST doesn't have: comments, blank lines, and line breaks that depend on
how long a line is.

Printing from the AST is what makes the style the same for everyone: the
output depends only on the program, not on how it was laid out. Two programs
that differ only in spacing and line breaks have the same AST, so they
format the same. `tests/fmt.rs` checks this for every program in `tests/t/`:
its tokens all on one line, and one token per line, must format to the same
text.

The AST does lose some things the source has. Parentheses are one: the
parser drops `(1 + 2)`'s and keeps the `1 + 2` inside. So the printer
decides where they go itself, and writes only the ones the meaning needs,
plus one kind more, `&&` inside `||`: `(a && b) || c`, as Prettier does,
since few people remember which of the two binds tighter.

## The layout algorithm

Philip Wadler's *A prettier printer* separates *what* a layout can be from
*which* layout to pick. The printer builds a `Doc` (`src/doc.rs`), a tree
of:

- `Text`: some text.
- `Line`: a space, or a line break.
- `Nest`: lines inside it are indented one level more.
- `Group`: a part that's printed *flat*, with every `Line` in it as a space,
  if all of it fits on the current line, and is otherwise *broken*, with
  every `Line` directly inside it a line break.

A call `f(a, b)` is a group: `f(`, then nested `SoftLine a, Line b,` (a
`SoftLine` is nothing when flat), then `SoftLine )`. Flat, that's
`f(a, b)`. Broken, it's:

```rust
f(
    a,
    b,
)
```

with a comma after the last argument too, from an `IfBreak(",", "")`: text
that appears only if its group breaks. Groups nest, and each decides for
itself. `render` goes through the doc in order, and at each group it asks
`fits`: would this group, flat, followed by what comes after it up to the
next possible line break, fit in what's left of the line? So the outermost
group that doesn't fit breaks first, and the ones inside get the whole of
their new lines to fit in:

```rust
print(compute_something_long(
    first_argument,
    second_argument,
    third_argument,
    fourth_argument,
    fifth,
));
```

Wadler's original is lazy: a group is a choice between two documents, and
the printer keeps the choices open, picking the best layout of the whole
thing. That's elegant in Haskell and slow anywhere else. T's engine is the
strict version from Christian Lindig's *Strictly Pretty*, which is also
what Prettier, the JavaScript formatter, runs: decide each group when you
reach it, by looking ahead only to the end of the current line. It never
backtracks, so it runs in time linear in the size of the program. The cost
is that a choice can't be undone when something later on the line doesn't
fit, which is why `fits` looks past the group to the next line break: the
`;` and `)` after a call count.

### The rules

The doc builder (`src/pretty.rs`) decides what's a group, and so what can
break:

- **Lists break all or nothing.** Arguments, parameters, struct fields and
  enum variants, type arguments and generics, closure parameters: on one
  line, or one per line with a comma after each.
- **Operators break before each operator in a run** of the same precedence,
  indented once, as in rustfmt: `a + b - c` becomes `a`, `+ b`, `- c` on
  three lines, and the `*` inside one operand stays with it.
- **A block goes on one line only if it's one expression that isn't itself
  a block**, as in `{ f(f(x)) }` or `if c { 1 } else { 2 }`. A block with a
  statement always breaks: `{ let y = x; y }` on one line hides the
  statement.
- **An `if` chain is one group**, so either the whole chain fits on one
  line or every block in it breaks. Half-broken chains are hard to read.
- **`match` arms are one per line, always.** Each arm's pattern can break
  (an or-pattern puts each alternative on its own line), and its body
  breaks like any expression.
- **If an `if` or `match` head breaks, its `{` goes on a line of its own**,
  so the condition doesn't run into the body.
- **A last argument that's a closure with a block, a block, `match`, `if` or
  struct literal breaks inside, not between arguments.** This is the one
  rule Wadler's groups can't express:

  ```rust
  apply(first_argument, second_argument, |x: int| -> int {
      let y = x * 2;
      y + 1
  });
  ```

  Breaking the argument list would put the closure on a line of its own, its
  body indented twice. `Doc::Hug` holds both layouts: the normal one, and the
  other arguments flat followed by the last argument broken. It picks the
  normal one if that fits flat, and otherwise the hugging one if its first
  line fits. A call whose only argument is a call does the same, which is
  how the `print(compute_something_long(` above stays on one line.

## Comments

The lexer used to drop comments: the parser never needs them. `t fmt` does,
so `lexer::lex_with_comments` keeps them, in a list of their own beside the
tokens. They're *trivia*, in the terminology of Roslyn and rust-analyzer:
part of the text, not of the program.

Then they have to go back in, at the right places, into output printed from
an AST that doesn't know about them. `Trivia` (`src/trivia.rs`) is a cursor
over them. The printer walks the AST in source order, and at each place a
comment can go, it asks for the comments up to that point in the source:

- **Leading:** before an item, statement, `match` arm or list element,
  every comment not printed yet that starts before it. Each goes on a line
  of its own.
- **Trailing:** after one of those, a comment on the same line with nothing
  but `,` or `;` in between, like `// expect: 25` above. It stays at the end
  of the line. So do comments lined up under it on the lines below, which is
  how `tests/t/block_like_stmts.t` writes several expected lines for one
  statement:

  ```rust
  print(h(true)); // expect: 1
                  // expect: 2
  ```

- **Dangling:** before the `}` or `)` that ends a block or list, the ones
  left, so a comment at the end of a block stays inside it.

Each comment is handed out once and the cursor only moves forward, so
comments come out in the order they went in, and none can be lost or
printed twice. A comment somewhere the printer never asks, like between
`1 +` and `2`, comes out at the next place it does, on a line of its own
after the statement. It moves, but it's kept.

A comment forces the groups around it to break: anything after a `//` on
the same line would be part of the comment. In the doc, a trailing comment
is a `Comment`, which `render` holds back and puts at the end of the line
when the line ends. Comments don't count toward the line width, so adding
or editing one never changes how the code around it is laid out.

### Lining up comments at line ends

Comments at the ends of consecutive lines are lined up, one space after the
longest line:

```rust
print(add5(1));                       // expect: 6
print(compose(add5, |x| x + 100, 0)); // expect: 105
```

T's tests are full of these, and a run of `// expect:` lines reads as a
table, code on the left and what it prints on the right. A table needs a
straight right-hand column. But lining things up has a cost: a long line
moves its neighbours' comments, and a one-line change becomes a diff of
several. So the runs are short. A line without such a comment, or a blank
line, ends the run, as in gofmt. And a line that would push some comment in
the run past 100 columns starts a new one, as in clang-format, so lining up
never makes a line too long. That's done after layout, on the finished
lines (`align_comments`).

### Blank lines

The AST doesn't record blank lines either, and they carry meaning: they
group statements into steps, and items into sections. `Trivia::blank_before`
looks at the source between something and the token or comment before it.
A blank line there is kept, and several become one. None is kept after a
`{` or before a `}`. Everything else about where lines go is up to the
formatter.

## Refusing to change the program

A formatter that changes what a program means, even once, can't be trusted
to run on save. So before `t fmt` writes anything, it parses its own output
and compares: the AST must be the same as the input's (ignoring spans, which
are positions in the text), and so must the list of comments. If not, it
refuses, and leaves the file alone. `tests/fmt.rs` runs this on every program
in `tests/t/` and `examples/`, and checks that formatting is *idempotent*:
formatting the output again changes nothing. A formatter that isn't
idempotent would make `--check` fail on files it had just formatted. And
`examples/` must already be formatted, so CI fails if they aren't.

## What was hard

- **Comments.** They aren't in the AST, and a program printed from an AST
  has no obvious place to put them back. Most of `src/trivia.rs` is
  deciding which node a comment belongs to. It needs positions the AST
  doesn't have (where the `fn` before a name starts, where a struct ends,
  where a list's `)` is), so `Trivia` keeps the tokens and finds them there.
- **What the AST forgets.** The AST splits a program into its types and its
  functions, so the formatter puts the items back in source order by
  position. Parentheses are gone, so the printer has to know exactly which
  are needed: an `if` at the start of a statement ends the statement, so
  `(if c { 1 } else { 2 }) - x` needs its parentheses, and a comparison on
  the left of another comparison needs them because comparisons don't
  chain. The printer for `t mono` got that last one wrong: it printed
  `(a < b) < c` without them, which doesn't parse. No test had such a
  program.
- **Printing what was written.** The same check caught a range pattern
  printed with the helper that formats them for error messages, which tidies
  `0..1` into `0`. Fine in a message, a different program in a file.
- **Hugging.** Breaking inside the last argument isn't a group: it's a
  second kind of choice, which commits to a broken layout because of how
  its first line looks. Getting `fits` to measure the other arguments flat
  and the last one broken took a dedicated measure (`first_line_fits`).
- **Comments that end a line in a group.** A comment on its own line,
  dangling in an empty argument list, printed as `g(// lone)`, which
  comments out the `)`. Every comment now forces the groups around it to
  break (`Doc::BreakParent`, after Prettier's `breakParent`). The check
  before writing caught this one too.
- **Choosing the rules.** The algorithm is short; the rules for what's a
  group are where the style lives, and each one was a judgement call:
  whether `{ x }` may stay on one line, where an operator goes when a line
  breaks, when a long line should move other lines' comments.

## Editors

`t lsp` answers `textDocument/formatting` with one edit that replaces the
whole file with what `t fmt` would write, or with no edits if it's
formatted. A file that doesn't parse gets none: the syntax error is already
showing. See the README for Neovim and Helix.

## Design decisions worth noticing

- **No options, and the editor's are ignored.** See above.
- **The formatter is the `t mono` printer.** One printer for both means
  `t mono` output is formatted T, and the round-trip tests for one test the
  other.
- **Print from the AST, keep only blank lines and comments from the
  source.** gofmt prints from the AST too, but keeps some of the author's
  line breaks; clang-format instead edits the whitespace between tokens.
  Both keep more of the author's layout, which is what a formatter with one
  style shouldn't.
- **Check the output, not just test the formatter.** The check runs on
  every format, in users' hands, not only on the programs the tests have.
- **A trailing comma whenever a list breaks**, so adding an element to a
  broken list is a one-line diff.

## Deliberately missing

| Missing | Why |
|---|---|
| options, a config file | the point |
| `// fmt: off` regions | an option by another name; gofmt has none either |
| keeping parentheses that aren't needed | they aren't in the AST |
| `textDocument/rangeFormatting` | the printer formats whole files from the AST |
| `t fmt` on stdin | not needed yet; `--check` covers CI |
| filling lines (`0 \| 1 \| 2` packed several per line) | one per line is plainer; see the exercises |

## Exercises

1. Why does `fits` look past the end of the group it's measuring? Write a
   statement that would come out over 100 columns if it didn't.
2. Find a program where the strict algorithm picks a worse layout than
   Wadler's lazy one would. (Hint: something late on a line that only
   fits if an earlier group breaks.)
3. Wadler's paper has `fill`, which packs as many items on each line as
   fit, like words in a paragraph. Add it to `Doc`, and use it for or-patterns
   of ints. Is it better for variant patterns too?
4. The check before writing compares ASTs and the order of comments. Name a
   bug in `t fmt` it wouldn't catch.
5. Keep parentheses the programmer wrote. What has to change in the parser
   and the AST, and what in the checker, which shouldn't care?
6. `rangeFormatting` asks for part of a file. Why is that harder for a
   formatter that prints from the AST than for one that edits whitespace?
   Design it anyway.
7. Line up `// expect:` comments across lines without comments, as
   `tests/t/closures.t` does by hand. Change one line in the middle of
   `main` and count the lines in the diff.

## Reading

- Philip Wadler, *A prettier printer*, 1997 (in *The Fun of Programming*,
  2003).
- Christian Lindig, *Strictly Pretty*, 2000. The strict version.
- Derek Oppen, *Prettyprinting*, TOPLAS 1980. The first linear-time
  algorithm.
- John Hughes, *The design of a pretty-printing library*, 1995.
- James Long, *A Prettier JavaScript Formatter*, 2017. Prettier, and
  `conditionalGroup`, which `Doc::Hug` is a special case of.
- Robert Griesemer, *The Cultural Evolution of gofmt*, 2015. Why one style.
- Bob Nystrom, *The Hardest Program I've Ever Written*, 2015. On `dartfmt`,
  and why line breaking is a search problem.
- *The Rust Style Guide*, which rustfmt implements, and where many of T's
  choices come from.
