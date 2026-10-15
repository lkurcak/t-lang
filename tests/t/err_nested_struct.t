// Only `fn` can be declared in a block, for now.
fn main() {
    struct P { x: int }
}
// error: a `struct` can't be declared in a block, only at the top level
