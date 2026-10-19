// `Point` from the module around it needs `super::` too.
#[derive(Discard)]
struct Point { x: int }

mod m {
    pub fn f(p: Point) -> int { 1 }
}

fn main() {}
// error: unknown type `Point`
