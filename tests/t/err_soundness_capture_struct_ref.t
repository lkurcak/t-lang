// A closure that captures can't be given to a variable from further out,
// that it could outlive.
#[derive(Discard, Copy)]
struct R<'a> { r: &'a int }

fn main() {
    let f;
    {
        let n = Box::new(5);
        let r = R { r: &*n };
        f = || *r.r;
    }
    print(f());
}
// error: cannot assign a closure that captures `r`
