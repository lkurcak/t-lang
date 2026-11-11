// A field's refinement names only the fields before it.
#[derive(Copy, Discard)]
enum Key { Range({ lo: u8 | lo < hi }, { hi: u8 | hi < 100 }) }

fn main() {}
// error: a field's refinement can only name the fields before it, and `hi` comes after field 0
