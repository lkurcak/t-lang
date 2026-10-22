// A trait's method would need each impl to pick a type, as an associated
// type does: not yet.
trait Make {
    fn make(&self) -> impl Fn() -> i64;
}

fn main() {}
// error: `impl` types can't be in a trait's methods yet
