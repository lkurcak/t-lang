// Two types of the same name in one block.
fn main() {
    struct P { x: int }
    enum P { A }
}
// error: type `P` is already defined
