// The same for a method's argument.

use alloc::boxed::Box;

#[derive(Discard)]
struct S {}

impl S {
    fn get<'a>(&self, r: &'a i64) -> &'a i64 { r }
}

fn main() {
    let b = Box::new(1);
    let s = S {};
    let r = s.get(&b);
    print(*r);
}
// error: expected `i64`, found `Box<{integer}>`
