// The only references a const holds are to strings and bytes, which live
// in the program.
const R: &u8 = &1;
// error: a const can't be of type `&u8`

fn main() {}
