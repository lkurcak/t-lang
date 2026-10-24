// `&mut b` used as a `&&i64` borrows what's in the box, `&'x i64`, so the
// result can't outlive `'x`. The borrow checker used to relate `&mut
// Box<&'x i64>` to `&&'y i64` part by part, skip the parts that didn't
// line up, and so lose `'x: 'y`: `out` dangled.
fn inner<'y>(r: &&'y i64) -> &'y i64 { *r }

fn main() with alloc {
    let out;
    let mut b: Box<&i64>;
    {
        let n = Box::new(5);
        b = Box::new(&*n);
        out = inner(&mut b);
    }
    print(*out);
}
// error: `n` doesn't live long enough
