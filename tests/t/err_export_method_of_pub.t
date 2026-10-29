// A method is seen no further than its type.
#[derive(Discard)]
pub struct S {}

impl S {
    export fn m(&self) -> i64 { 1 }
}

fn main() { let _x = S {}.m(); }
// error: method `m` can't be exported: `S` isn't
