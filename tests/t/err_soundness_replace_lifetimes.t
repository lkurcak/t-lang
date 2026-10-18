// The same for `replace`.
fn main() {
    let long = Box::new(1);
    let mut a = &*long;
    {
        let short = Box::new(2);
        let old = replace(&mut a, &*short);
        print(*old);
    }
    print(*a);
}
// error: `short` doesn't live long enough
