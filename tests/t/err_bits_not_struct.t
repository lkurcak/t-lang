#[derive(Copy, Discard)]
struct S {}

fn main() {
    let _x = !S {};
}
// error: `!` works on `bool` and integers, not on `S`
