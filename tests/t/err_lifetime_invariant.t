// `&mut T` is invariant in `T`: a `&mut &'long i64` can't be used as a
// `&mut &'short i64`, or a short-lived reference could be written where a
// long-lived one is expected.
fn assign<'a>(r: &mut &'a i64, v: &'a i64) { *r = v; }

fn main() {
    let x = 1;
    let mut r = &x;
    {
        let y = 2;
        assign(&mut r, &y);
    }
    print(*r);
}
// error: `y` doesn't live long enough
