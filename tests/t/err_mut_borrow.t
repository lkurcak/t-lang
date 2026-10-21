fn bump(n: &mut int) with panic { *n += 1; }

fn main() with panic {
    let x = 1;
    bump(&mut x);
}
// error: cannot borrow `x` as mutable, as `x` isn't declared `mut`
