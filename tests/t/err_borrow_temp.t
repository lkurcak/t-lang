use alloc::boxed::Box;

fn main() with alloc {
    let r = &*Box::new(1i64);
    print(*r);
}
// error: a temporary value is dropped here while still borrowed
