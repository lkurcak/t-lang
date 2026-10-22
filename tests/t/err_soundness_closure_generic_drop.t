// Calling `f` borrows it, so it's still there at the end, and an `F` may
// not be droppable: a closure that owns a value that must be used up isn't.
fn apply<F: Fn() -> i64>(f: F) -> i64 { f() }

fn main() {
    print(apply(|| 1));
}
// error: `f` is never used up
