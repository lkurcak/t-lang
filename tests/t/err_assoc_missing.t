// An impl must say what each of the trait's associated types is.
trait Tr {
    type Out;
}

impl Tr for i64 {}

fn main() {}
// error: this impl of `Tr` for `i64` is missing an associated type: `Out`
