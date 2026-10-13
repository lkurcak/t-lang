// Same fields, different names: different types (nominal typing).
struct Meters { v: int }
struct Feet { v: int }
fn main() {
    let m: Meters = Feet { v: 3 };
}
// error: expected `Meters`, found `Feet`
