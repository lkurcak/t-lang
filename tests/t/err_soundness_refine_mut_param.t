// A `mut` parameter can change, so its refinement is an invariant, checked
// at each assignment.
fn f(mut x: { v: int | v > 0 }) -> int {
    x = 0;
    10 / x
}

fn main() with panic {
    print(f(1));
}
// error: can't prove `v > 0` for `x`
