// Each step to work out `<W<T> as Tr>::Out` needs it for a bigger type.
#[derive(Discard)]
struct W<T> { t: T }

trait Tr {
    type Out;
}

impl<T> Tr for W<T> {
    type Out = <W<W<T>> as Tr>::Out;
}

fn main() {}
// error: smaller
