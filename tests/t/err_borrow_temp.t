fn main() with {alloc, panic} {
    let r = &*Box::new(1);
    print(*r);
}
// error: a temporary value is dropped here while still borrowed
