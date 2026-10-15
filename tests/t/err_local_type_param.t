// A local type can't use the type parameters of the function it's in: it
// means what it would at the top level.
fn wrap<T>(x: T) -> T {
    struct Box { v: T }
    x
}

fn main() {
    print(wrap(1));
}
// error: can't use type parameter `T` of the enclosing function in a local `struct`; declare it on this one too, as in `struct Box<T>`
