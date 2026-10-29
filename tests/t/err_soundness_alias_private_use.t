// And other modules can't use it.
mod m {
    type Num = i64;
}

fn main() { let _x: m::Num = 1; }
// error: `Num` is private to `m`
