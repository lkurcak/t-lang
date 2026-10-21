// The other way round, too: assigning the `pub` field `lo` would need
// what's known of the private `hi`.
pub struct Span { pub lo: int, hi: { h: int | h >= lo } }

fn main() {}
// error: the refinement of `hi`, which isn't `pub`, can't name `lo`, which is: a field's refinement can only name fields as visible as it
