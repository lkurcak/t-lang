// A closure that takes a reference of any lifetime can't keep it: the next
// call might give it one that dies first. Here it would store `&b` in
// `keep`, which is read after `b` is gone.
fn main() {
    let a = 1;
    let mut keep = &a;
    let mut f = |x: &int| { keep = x; };
    {
        let b = 2;
        f(&b);
    }
    print(*keep);
}
// error: this closure stores a reference from `x` in `keep`
