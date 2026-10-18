// An impl must say what each of the trait's associated types is.
trait Tr {
    type Out;
}

impl Tr for int {}

fn main() {}
// error: this impl of `Tr` for `int` is missing an associated type: `Out`
