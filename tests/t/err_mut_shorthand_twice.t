#[derive(Discard)]
struct P { x: i64 }

fn main() {
    let P { mut mut x } = P { x: 1 };
}
// error: expected a field name after `mut`
