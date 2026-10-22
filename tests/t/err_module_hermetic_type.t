// `Point` from the module around it needs `super::` too.
#[derive(Discard)]
struct Point { x: i64 }

mod m {
    pub fn f(p: Point) -> i64 { 1 }
}

fn main() {}
// error: unknown type `Point`
