#[derive(Discard)]
struct P { x: i64 }

fn main() {
    let p = P { x: 1 };
    p.x = 2;
}
// error: cannot assign to `p.x`, as `p` isn't declared `mut`
