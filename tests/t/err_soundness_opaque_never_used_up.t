// What `once` returns owns a `Box` and isn't `Discard`: it must be called.
fn once(b: Box<i64>) -> impl FnOnce() -> i64 { move || *b }

fn main() with {alloc, panic} {
    let f = once(Box::new(1));
}
// warning: unused variable `f`: use it up
// error: `f` is never used up
