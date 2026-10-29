// What an exported function takes or gives must be exported too, or
// other packages couldn't write it down.
#[derive(Discard)]
pub struct Hidden { n: i64 }

export fn make() -> Hidden { Hidden { n: 1 } }

fn main() { let _h = make(); }
// error: `make` is exported, but its signature names `Hidden`, which isn't exported
