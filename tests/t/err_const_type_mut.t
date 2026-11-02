// A const is plain data: no `&mut`, which would be one place for every use.
const R: &mut u8 = &mut 1;
// error: a const can't be of type `&mut u8`

fn main() {}
