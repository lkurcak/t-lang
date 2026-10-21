// A field's refinement names only fields as visible as it: who can assign
// `hi` must be able to see what it has to keep.
pub struct Span { lo: int, pub hi: { h: int | h >= lo } }

fn main() {}
// error: the refinement of `hi`, which is `pub`, can't name `lo`, which isn't: a field's refinement can only name fields as visible as it
