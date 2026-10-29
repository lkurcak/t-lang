// `m::P` is `m`'s `Point`, not the one where it's used.
mod m {
    pub struct Point { pub x: i64 }
    pub type P = Point;
}

struct Point { x: bool }

fn main() {
    let _p: m::P = Point { x: true };
}
// error: expected `Point`, found `Point`; these are two different types called `Point`, one declared at the top level and one in `m`
