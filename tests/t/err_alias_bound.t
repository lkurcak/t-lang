// A bound on an alias's parameter would be a promise nothing checks. What
// it stands for is checked where it's used, as if written there.
type Sorted<T: Ord> = alloc::list::List<T>;

fn main() {}
// error: a type alias's parameters can't have bounds; what it stands for is checked where it's used
