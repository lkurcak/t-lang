// A const is plain data: a tuple holding a `&mut` isn't.
const C: (u8, &mut u8) = (1, &mut 2);

fn main() {}
// error: a const can't be of type `(u8, &mut u8)`
