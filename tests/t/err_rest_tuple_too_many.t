// The same of a tuple's values.
fn f(t: (u8, u8)) -> u8 {
    let (a, b, c, ..) = t;
    a + b + c
}

fn main() {
    let _ = f((1, 2));
}
// error: this pattern is for a tuple of at least 3 values, but the value is `(u8, u8)`
