// Same fields, different names: different types (nominal typing).
#[derive(Discard)]
struct Meters { v: i64 }
#[derive(Discard)]
struct Feet { v: i64 }
fn main() {
    let m: Meters = Feet { v: 3 };
}
// error: expected `Meters`, found `Feet`
