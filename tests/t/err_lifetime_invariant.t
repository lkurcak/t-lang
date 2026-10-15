// `&mut T` is invariant in `T`: a `&mut &'long int` can't be used as a
// `&mut &'short int`, or a short-lived reference could be written where a
// long-lived one is expected.
fn assign<'a>(r: &mut &'a int, v: &'a int) { *r = v; }

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
