fn id<T>(x: T) -> T { x }
fn main() { print(id::<i64, bool>(1)); }
// error: `id` takes 1 type argument but 2 were supplied
