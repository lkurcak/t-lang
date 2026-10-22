// Using a generic function as a value instantiates it once, so `f` has one
// type: there's no let-generalisation.
fn id<T>(x: T) -> T { x }
fn main() {
    let f = id;
    print(f(1));
    print_bool(f(true));
}
// error: expected `{integer}`, found `bool`
