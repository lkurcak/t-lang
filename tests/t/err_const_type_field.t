// A struct is plain data if its fields are.
struct Holder<'a> { r: &'a mut u8 }

const H: Holder<'static> = Holder { r: &mut 1 };
// error: a const can't be of type `Holder<'static>`

fn main() {}
