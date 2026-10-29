// A parameter an alias doesn't use couldn't be worked out from anything.
type Count<T> = i64;

fn main() {}
// error: type alias `Count` doesn't use its parameter `T`
