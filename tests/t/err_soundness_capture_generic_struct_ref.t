// A `Copy` struct holding a reference can't be captured, even through a
// type argument.
#[derive(Discard, Copy)]
struct W<T> { x: T }

fn main() {
    let n = Box::new(5);
    let w = W { x: &*n };
    let f = || *w.x;
    print(f());
}
// error: this closure can't capture `w`: it's a `W<&int>`, which holds a reference
