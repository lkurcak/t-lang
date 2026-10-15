fn id<T>(x: T) -> T { x }
fn main() { print(id::<int, bool>(1)); }
// error: `id` takes 1 type argument but 2 were supplied
