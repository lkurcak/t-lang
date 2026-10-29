// `&mut` is invariant in its referent, through an alias too: a
// `Cell<'static>` can't hold a shorter borrow.
type Cell<'a> = &'a mut &'a i64;

fn put<'a>(c: Cell<'a>, r: &'a i64) { *c = r; }

fn main() {
    let mut keep: &'static i64 = &1;
    {
        let x = 2i64;
        put(&mut keep, &x);
    }
    let _y = *keep;
}
// error: `x` doesn't live long enough: it goes out of scope here while still borrowed
