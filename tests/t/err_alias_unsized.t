// An alias stands for a type with a size; a reference to a slice has one.
type Bytes = [u8];

fn main() {}
// error: a type alias can't stand for `[u8]`, which has no size; alias a reference to it instead
