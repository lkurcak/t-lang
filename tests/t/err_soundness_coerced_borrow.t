// `&mut b` used as a `&&int` borrows what's in the box, `&'x int`, so the
// result can't outlive `'x`. The borrow checker used to relate `&mut
// Box<&'x int>` to `&&'y int` part by part, skip the parts that didn't
// line up, and so lose `'x: 'y`: `out` dangled.
fn inner<'y>(r: &&'y int) -> &'y int { *r }

fn main() {
    let out;
    let mut b: Box<&int>;
    {
        let n = Box::new(5);
        b = Box::new(&*n);
        out = inner(&mut b);
    }
    print(*out);
}
// error: `n` doesn't live long enough
