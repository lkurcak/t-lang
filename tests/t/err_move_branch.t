// Moved in one branch, so maybe moved after.
fn main() with alloc {
    let b = Box::new(1i64);
    if true {
        let c = b;
        print(*c);
    }
    print(*b);
}
// error: use of moved value `b`
