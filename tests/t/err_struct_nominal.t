// Same fields, different names: different types (nominal typing).
#[derive(Discard)]
struct Meters { v: int }
#[derive(Discard)]
struct Feet { v: int }
fn main() {
    let m: Meters = Feet { v: 3 };
}
// error: expected `Meters`, found `Feet`
