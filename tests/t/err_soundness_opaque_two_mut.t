// Each closure holds a `&mut n`, for as long as it's around.
fn bump(r: &mut int) -> impl FnMut() + Discard { move || *r += 1 }

fn main() {
    let mut n = 0;
    let mut a = bump(&mut n);
    let mut b = bump(&mut n);
    a();
    b();
    print(n);
}
// error: cannot borrow `n` as mutable more than once at a time
