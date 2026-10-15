fn bump(n: &mut int) { *n += 1; }

fn main() {
    let x = 1;
    bump(&mut x);
}
// error: cannot borrow `x` as mutable, as `x` isn't declared `mut`
