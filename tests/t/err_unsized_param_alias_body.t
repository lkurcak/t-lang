// An alias's `unsized` parameter is as any other.
type Pair<unsized T> = Option<T>;

fn main() {}
// error: `T` is `unsized`, so it can only be behind a reference, as in `&T`
