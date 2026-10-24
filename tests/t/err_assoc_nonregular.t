// A type whose field is of the type at a projection of its parameter:
// `S<T>` needs `S<T::Out>`, which may be bigger, and so on.

use alloc::boxed::Box;

trait Tr {
    type Out;
}

enum S<T: Tr> { End, More(Box<S<T::Out>>) }

fn main() {}
// error: polymorphic recursion
