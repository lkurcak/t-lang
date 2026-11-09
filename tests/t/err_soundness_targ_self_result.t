// Nor one whose method gives back a `Self` from a `&self`: it can make one.
trait Dup {
    fn dup(&self) -> Self;
}

impl Dup for usize {
    fn dup(&self) -> usize { 9 }
}

fn copy_of<T: Dup>(x: &T) -> T { x.dup() }

fn main() {
    let a: [u8; 4] = [1, 2, 3, 4];
    let i: { v: usize | v < 4 } = 1;
    let j: { v: usize | v < 4 } = copy_of(&i);
    let _ = a[j];
}
// error: can't prove `v < 4` for `j`
