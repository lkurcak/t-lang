// A path in `#[derive(..)]` names a trait like any path does, and here
// `crate::Eq` is this program's, which can't be derived. (A bare `Eq`
// would be core's, whatever's in scope: see derive_debug_any_scope.t.)
trait Eq {}

#[derive(crate::Eq)]
struct S {}

fn main() {}
// error: only `core`'s `Copy`, `Debug`, `Discard`, `Eq`, `Ord`, `Send` and `Sync` can be derived, and `crate::Eq` here is `crate::Eq`
