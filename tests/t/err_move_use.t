fn main() with {alloc, panic} {
    let b = Box::new(1);
    let c = b;
    print(*c);
    print(*b);
}
// error: use of moved value `b`
