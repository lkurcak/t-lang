// A `&mut` isn't `Copy`, so neither is a tuple holding one.
fn main() {
    let mut n: u8 = 1;
    let t = (&mut n, 2u8);
    let a = t;
    let b = t;
    *a.0 = 3;
    *b.0 = 4;
}
// error: use of moved value `t`
