// The root of a borrowed place, `id_ref(r)`, is worked out in order, so it
// reads `r` before the next argument changes it; but `r` has one type for
// as long as it lives, so it can't be given a reference to `tmp`, which
// doesn't live as long as what came out of `id_ref`.
fn id_ref(r: &Box<int>) -> &Box<int> { r }

fn f(x: &int, n: int) -> int with panic { *x + n }

fn main() with {alloc, panic} {
    let b1 = Box::new(1);
    let mut r = &b1;
    print(f(&**id_ref(r), {
        let tmp = Box::new(9);
        r = &tmp;
        0
    }));
}
// error: `tmp` doesn't live long enough
