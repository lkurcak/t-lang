// The name is the iterator's, which the head makes: it isn't in scope in
// the head.
fn main() {
    let a = [1u8, 2];
    for _x in it: it.iter() {}
}
// error: unknown variable `it`
