// Comparing field by field needs every field to compare, and functions
// can't.
#[derive(Discard, Eq)]
struct Handler { f: fn(i64) -> i64 }
fn id(x: i64) -> i64 { x }
fn main() -> bool {
    Handler { f: id } == Handler { f: id }
}
// error: `Handler` can't be `Eq`: its field `f` holds a `fn(i64) -> i64`, which isn't (functions can't be compared)
