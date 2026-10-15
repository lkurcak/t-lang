// A local type's name is only in scope in its block.
fn main() {
    {
        struct P { x: int }
    }
    let p = P { x: 1 };
}
// error: unknown type `P`
