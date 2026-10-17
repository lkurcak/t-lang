// Comparing field by field needs every field to compare, and functions
// can't.
#[derive(Eq)]
struct Handler { f: fn(int) -> int }
fn id(x: int) -> int { x }
fn main() -> bool {
    Handler { f: id } == Handler { f: id }
}
// error: `Handler` can't be `Eq`: its field `f` holds a `fn(int) -> int`, which isn't (functions can't be compared)
