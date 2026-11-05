// What an impl says of its own type's associated type says nothing of
// another type's: inside the impl for `Wrap<T>`, `T::Out` is still rigid,
// whatever `Wrap<T>::Out` is.
trait Tr {
    type Out;
    fn get(&self) -> Self::Out;
}

struct Wrap<T> { t: T }

impl<T: Tr> Tr for Wrap<T> {
    type Out = u8;
    fn get(&self) -> u8 { self.t.get() }
}

fn main() {}
// error: expected `u8`, found `T::Out`
