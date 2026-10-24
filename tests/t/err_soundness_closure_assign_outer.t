// A closure given to a variable from further out can't outlive what it
// borrows.
#[derive(Discard, Copy)]
struct R<'a> { r: &'a i64 }

fn main() with alloc {
    let f;
    {
        let n = Box::new(5);
        let r = R { r: &*n };
        f = || *r.r;
    }
    print(f());
}
// error: `r` doesn't live long enough
