# t-lang: notes for agents

## Backlog

What's next is tracked in `BACKLOG.md` at the repo root. It's shown as it stands on the
default branch, so it has to be true there.

- Sections: a `# Backlog` title, then `## Now` (one ordered list, in order), `## Later`,
  `## Done`. Later may be grouped under `###` headings; Now may not. Every item under Now
  may be numbered `1.`, so that removing one doesn't renumber the rest.
- An item is one list item of a line or two, and something that can be finished. If it needs
  more, it links to a details file as a plain Markdown link
  (`1. [Jump and fast fall](backlog/jump.md)`), deleted when the item is done. A standing
  practice (read the feedback first, audit the data) isn't an item: it goes in the repo's
  `CLAUDE.md`.
- An item that can't start before a date starts with it: "From 17 Dec: …". An item only the
  owner can do (a decision, a key, a setting on their account) starts with "Owner:".
- Keep it current: move what you finished to Done in the same change, and put new ideas
  under Later instead of leaving them in the chat. Don't invent items.
- Done is newest first, a line each, ending with the pull request's number where there is
  one (`(#63)`). It keeps about the last ten; git has the rest.
- Name an item by its words or its details file, not by its number: numbers shift when an
  item above it is done. Told a number, say the item's title back before you start.
- On "go", read `BACKLOG.md` and continue with the first item under Now that you can do:
  not the owner's, and not one still waiting for its date. Tell the owner which of theirs
  are waiting.

T is a project that spans several repos (t-lang, t-libs, tcalc, t-nonograms, t-maze,
t-metal, t-life, t-fmt); this is its main repo and holds the one backlog.

## Decisions

The language's decisions are in each step's doc, `docs/NN-*.md`, under its own
`## Decisions`. Decisions about the repo itself:

- Every function in core, alloc and std that isn't exported says its `with`, `with {}`
  where it has no effects (retrospective 20, 0.109.1). One that says none has its effects
  inferred, so it's checked in full on every `t check` rather than only typed:
  `core::float::dec`'s took an empty program's check from about 100 ms to 180. The test
  `library_functions_say_their_effects` lists any that don't.

## Math functions

A change to `core/float/math.t` or `core/float/fix.t` is checked against MPFR before it
merges, as CI can't tell a fast path that's a little off (its vectors are the hardest of
random inputs, which a fast path rounds right even with its rounding test turned off): every
input of each `f32` function it touches, `scripts/math/check.sh <fn> f32 all`, and millions
of `f64` ones, `... f64 bits N` and `range`. Regenerate `tests/math/vectors.txt` with
`scripts/math/vectors.c` if a function's inputs or answers change.
