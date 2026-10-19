// What `once` returns owns a `Box` and isn't `Discard`: it must be called.
fn once(b: Box<int>) -> impl FnOnce() -> int { move || *b }

fn main() {
    let f = once(Box::new(1));
}
// warning: unused variable `f`: use it up
// error: `f` is never used up
