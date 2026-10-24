// Deriving `Discard` isn't a use of the type.
#[derive(Discard)]
struct Unused { n: i64 } // warning: struct `Unused` is never constructed

fn main() {}
