// The same for a generic impl: `W<T>` is `Discard` only if `T` is, and the
// impl doesn't say it is.
#[derive(Discard)]
struct W<T> { x: T }

trait Sub: Discard {}

impl<T> Sub for W<T> {}

fn main() {}
// error: `W<T>` doesn't implement `Discard`, which `Sub` needs, as it's `trait Sub: Discard`
