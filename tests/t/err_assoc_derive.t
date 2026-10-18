// A derived impl can rely on what the trait says of `Out`, and nothing
// else: here it says nothing.
trait Tr {
    type Out;
}

#[derive(Discard)]
struct H<T: Tr> { f: T::Out }

fn main() {}
// error: `H` can't be `Discard`: its field `f` holds a `T::Out`, and `Tr` doesn't say that's `Discard`; declare it as `type Out: Discard;`
