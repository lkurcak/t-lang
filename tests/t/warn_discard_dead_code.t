// Deriving or implementing `Discard` isn't a use of the type.
#[derive(Discard)]
struct Unused { n: i64 } // warning: struct `Unused` is never constructed

struct Unwritten { n: i64 } // warning: struct `Unwritten` is never constructed

impl Discard for Unwritten {}

fn main() {}
