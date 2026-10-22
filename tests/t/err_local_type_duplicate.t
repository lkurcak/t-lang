// Two types of the same name in one block.
fn main() {
    #[derive(Discard)]
    struct P { x: i64 }
    #[derive(Discard)]
    enum P { A }
}
// error: type `P` is already defined
