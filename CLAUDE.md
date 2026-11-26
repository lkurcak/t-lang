# t-lang: notes for agents

## Backlogs

My projects track what's next in `BACKLOG.md` at the repo root. almanac reads it from the
default branch and shows it, so it has to be true.

- Format: a `# Backlog` title, then `## Now` (an ordered list, in order), `## Later`,
  `## Done`, one list item each. An item that needs more than a line or two links to its
  details file as a plain Markdown link (`1. [Jump and fast fall](backlog/jump.md)`); delete
  that file when the item is done.
- If the repo has one, keep it current: move what you finished to Done in the same change,
  and put new ideas under Later instead of leaving them in the chat.
- If it has none and the work is more than a one-off fix, start one from what I've said is
  next, and copy this section into the repo's `CLAUDE.md` so it holds without this file.
  Don't invent items: a short backlog is fine.
- A project that spans several repos keeps one backlog, in its main repo.
- What's next lives in `BACKLOG.md`, not in Claude memory, where I can't read it and almanac
  can't see it. If this project's memory holds a roadmap, backlog, next-steps or handoff
  file, move what's still open in it into `BACKLOG.md`, then cut the memory file down to a
  pointer to `BACKLOG.md`. On "go", read `BACKLOG.md` and continue with the first item
  under Now. Memory keeps what isn't a work item: how I like to work, where things are.

T is a project that spans several repos (t-lang, t-libs, tcalc, t-nonograms, t-maze,
t-metal, t-life, t-fmt); this is its main repo and holds the one backlog.
