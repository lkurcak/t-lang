use alloc::boxed::Box;

fn main() with alloc {
    let b = Box::new(1i64);
    let c = b;
    print(*c);
    print(*b);
}
// error: use of moved value `b`
