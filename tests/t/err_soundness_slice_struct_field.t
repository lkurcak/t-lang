// Nor as a struct's field.
struct S { xs: [int] }

fn main() {}
// error: a slice `[int]` can only be behind a reference, as in `&[int]`
