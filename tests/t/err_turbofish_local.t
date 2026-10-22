fn id<T>(x: T) -> T { x }
fn main() {
    let f = id;
    print(f::<i64>(1));
}
// error: `f` is a variable, and only functions, structs and enums take type arguments
