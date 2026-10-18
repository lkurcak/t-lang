// Deriving or implementing `Discard` isn't a use of the type.
#[derive(Discard)]
struct Unused { n: int } // warning: struct `Unused` is never constructed

struct Unwritten { n: int } // warning: struct `Unwritten` is never constructed

impl Discard for Unwritten {}

fn main() {}
