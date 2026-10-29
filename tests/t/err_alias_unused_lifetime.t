// The same for a lifetime parameter.
type Count<'a> = i64;

fn main() {}
// error: type alias `Count` doesn't use its parameter `'a`
