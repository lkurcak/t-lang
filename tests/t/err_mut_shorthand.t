struct P { x: int }

fn main() {
    let P { mut x } = P { x: 1 };
}
// error: write `x: mut x` to bind field `x` mutably
