// A field's refinement names only fields as visible as it: who can assign
// `hi` must be able to see what it has to keep.
pub struct Span { lo: i64, pub hi: { h: i64 | h >= lo } }

fn main() {}
// error: the refinement of `hi`, which is `pub`, can't name `lo`, which is private: a field's refinement can only name fields exactly as visible as it
