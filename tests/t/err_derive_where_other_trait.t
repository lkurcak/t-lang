// What a derived `Discard` says it needs can only be `Discard`, as its
// fields can't need anything else.
pub trait Show {
    fn show(&self) -> i64;
}

#[derive(Discard where T: Show)]
pub struct W<T> { t: T }

fn main() {}
// error: a derived `Discard` can only need `Discard` of type parameters: that's all its fields can need
