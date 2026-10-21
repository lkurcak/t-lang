// `T::m` without a call is a function value too.
#[derive(Discard)]
struct S {}

impl S {
    fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }
}

fn main() {
    let f = S::div;
    print(f(1, 0));
}
// error: `div` can't be used as a value: its parameter `d` has a refinement, which a call through a value wouldn't check
