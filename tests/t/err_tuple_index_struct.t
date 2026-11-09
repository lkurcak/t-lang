// A struct's fields have names, not numbers.
#[derive(Discard)]
struct P { x: i64 }

fn main() {
    let p = P { x: 1 };
    let _x = p.0;
}
// error: `P` has no field `0`
