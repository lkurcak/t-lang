// `..` stands for none or more values, not fewer than none.
enum E {
    Two(u8, u8),
}

fn f(e: E) -> u8 {
    match e {
        E::Two(a, b, c, ..) => a + b + c,
    }
}

fn main() {
    let _ = f(E::Two(1, 2));
}
// error: `E::Two` has 2 fields, but this pattern names 3 besides `..`
