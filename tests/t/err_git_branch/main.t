// A branch moves, and without a lockfile what it meant would be lost.
// error: a dependency has only a `path`, or a `git` and a `rev`, not a `branch`: a branch or tag can move, so a `git` dependency is pinned by its `rev`, for now
fn main() {}
