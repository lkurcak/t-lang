// A parameter only under a projection isn't decided: `T::Out` doesn't say
// which `T` it's of.
trait Tr {
    type Out;
}

#[derive(Discard where F: Discard)]
struct W<F> { f: F }

trait Get {
    fn get(&self) -> i64;
}

impl<T: Tr, F: Fn() -> T::Out> Get for W<F> {
    fn get(&self) -> i64 { 1 }
}

fn main() {}
// error: type parameter `T` isn't in `W<F>`, nor decided by a bound
