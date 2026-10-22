// `move` copies `r` into the closure, but `r` is a reference: the closure
// still can't outlive what it points to.
fn main() {
    let c = {
        let n: i64 = 1;
        let r = &n;
        move || *r
    };
    print(c());
}
// error: `n` doesn't live long enough
