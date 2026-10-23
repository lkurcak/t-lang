// The message names the method a `?` returns early from.
#[derive(Discard)]
struct P { v: Option<i64> }

impl P {
    fn get(&self) -> i64 { self.v? }
}

fn main() { let _ = P { v: Option::None }; }
// error: `?` returns early from `get`, but its result, `i64`, doesn't implement `core::ops::Try`
