// `a` holds a `&mut n` until it's last used.
fn bump(r: &mut int) -> impl FnMut() + Discard { move || *r += 1 }

fn main() {
    let mut n = 0;
    let mut a = bump(&mut n);
    print(n);
    a();
}
// error: cannot use `n` while `n` is mutably borrowed
