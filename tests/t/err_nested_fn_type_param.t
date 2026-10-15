// A nested `fn` can't use the type parameters of the one it's in, either.
fn wrap<T>(x: T) -> T {
    fn id(y: T) -> T { y }
    id(x)
}

fn main() {
    print(wrap(1));
}
// error: can't use type parameter `T` of the enclosing function in a nested `fn`
