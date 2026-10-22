// Nor as a struct's field.
struct S { xs: [i64] }

fn main() {}
// error: a slice `[i64]` can only be behind a reference, as in `&[i64]`
