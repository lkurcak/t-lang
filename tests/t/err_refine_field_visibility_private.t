// The other way round, too: assigning the `pub` field `lo` would need
// what's known of the private `hi`.
pub struct Span { pub lo: i64, hi: { h: i64 | h >= lo } }

fn main() {}
// error: the refinement of `hi`, which is private, can't name `lo`, which is `pub`: a field's refinement can only name fields exactly as visible as it
