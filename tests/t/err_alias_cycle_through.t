// Through another alias, the same.
type A = Option<B>;
type B = alloc::boxed::Box<A>;

fn main() {}
// error: type alias `A` stands for a type that contains itself, through `B`; a recursive type needs a struct or enum
