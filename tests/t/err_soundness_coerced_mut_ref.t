// `&mut x` used as a `&int`, for `x` a `&int`: the result is what `x`
// points to, so it can't outlive that.
fn id<'y>(r: &'y int) -> &'y int { r }

fn main() {
    let out;
    let mut x: &int;
    {
        let n = Box::new(5);
        x = &*n;
        out = id(&mut x);
    }
    print(*out);
}
// error: `n` doesn't live long enough
