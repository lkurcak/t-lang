#[derive(Discard)]
struct P { x: int, y: int }

fn main() {
    let P { x, y };
}
// error: a `let` with no value can only declare a variable, like `let x;`, as assignment gives a variable its value
