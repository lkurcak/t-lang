// The same for a method's argument.
#[derive(Discard)]
struct S {}

impl S {
    fn get<'a>(&self, r: &'a int) -> &'a int { r }
}

fn main() {
    let b = Box::new(1);
    let s = S {};
    let r = s.get(&b);
    print(*r);
}
// error: expected `int`, found `Box<int>`
