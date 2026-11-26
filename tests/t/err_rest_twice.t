// One `..` per list: with two, which values each stands for isn't said.
fn f(t: (u8, u8, u8, u8)) -> u8 {
    let (a, .., b, ..) = t;
    a + b
}

fn main() {
    let _ = f((1, 2, 3, 4));
}
// error: only one `..`
