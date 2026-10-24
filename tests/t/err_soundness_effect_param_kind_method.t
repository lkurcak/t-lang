// An impl's method must have the trait method's kinds of parameters: if
// `m`'s `A` could be an effect parameter in the impl, the trait's callers
// would pass it a type.
trait Tr {
    fn m<A: Discard>(&self, a: A) -> i64;
}

#[derive(Discard)]
struct W {}

impl Tr for W {
    fn m<effect a>(&self, x: i64) -> i64 with a { x }
}

fn main() {}
// error: method `m` doesn't match its declaration in trait `Tr`: its `a` is an effect parameter, where the trait's `A` is a type parameter
