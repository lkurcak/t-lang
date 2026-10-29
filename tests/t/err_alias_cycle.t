// An alias that contains itself would never end: a recursive type needs a
// struct or enum, which gives it a name of its own.
type Chain = Option<Box<Chain>>;

fn main() {}
// error: type alias `Chain` stands for a type that contains itself; a recursive type needs a struct or enum
