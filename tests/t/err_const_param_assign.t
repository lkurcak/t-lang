// A const parameter is a value fixed by the caller, not a variable.
fn f<const N: usize>() {
    N = 3;
}

fn main() {}
// error: cannot assign to `N`, a const parameter
