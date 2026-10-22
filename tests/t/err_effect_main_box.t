// `main` says what the program may do, like any function.
fn main() {
    let b = Box::new(1i64);
    print(*b);
}
// error: `Box::new` may allocate, but `main` doesn't say `with alloc`
