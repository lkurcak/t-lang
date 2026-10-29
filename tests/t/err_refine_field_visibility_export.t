// A dependent that can write `hi` couldn't see the invariant it must keep,
// which names `lo`: so a field's refinement names only fields seen exactly
// as far.
#[derive(Discard)]
export struct Span { pub lo: i64, export hi: { h: i64 | h >= lo } }

fn main() {}
// error: the refinement of `hi`, which is exported, can't name `lo`, which is `pub`: a field's refinement can only name fields exactly as visible as it
