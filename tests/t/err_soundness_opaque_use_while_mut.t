// `a` holds a `&mut n` until it's last used.
fn bump(r: &mut i64) -> impl FnMut() with panic + Discard { move || *r += 1 }

fn main() with panic {
    let mut n = 0;
    let mut a = bump(&mut n);
    print(n);
    a();
}
// error: cannot use `n` while `n` is mutably borrowed
