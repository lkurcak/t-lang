// A field's refinement names only the fields before it.
struct Span { lo: { l: int | l <= hi }, hi: int }

fn main() {}
// error: a field's refinement can only name the fields before it, and `hi` comes after `lo`
