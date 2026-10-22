// A field's refinement names only the fields before it.
struct Span { lo: { l: i64 | l <= hi }, hi: i64 }

fn main() {}
// error: a field's refinement can only name the fields before it, and `hi` comes after `lo`
