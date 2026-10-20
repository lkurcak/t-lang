// Moved in one branch, so maybe moved after.
fn main() with {alloc, panic} {
    let b = Box::new(1);
    if true {
        let c = b;
        print(*c);
    }
    print(*b);
}
// error: use of moved value `b`
