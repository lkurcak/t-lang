#[derive(Discard)]
struct P { x: int }
fn main() {
    let p = P { x: 1 };
    print(p.z);
}
// error: `P` has no field `z`
