// Nor a field's.
struct S<unsized T> { x: T }

fn main() {}
// error: `T` is `unsized`, so it can only be behind a reference, as in `&T`
