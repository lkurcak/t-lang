// The closure holds a `&mut int`, which isn't `Copy`.
fn bump(r: &mut int) -> impl FnMut() + Copy + Discard { move || *r += 1 }

fn main() {
    let mut n = 0;
    let mut b = bump(&mut n);
    b();
}
// error: doesn't implement `Copy`, which `bump`'s return type says
