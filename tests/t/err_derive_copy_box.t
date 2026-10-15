#[derive(Copy)]
struct Owner { b: Box<int> }

fn main() {}
// error: `Owner` can't be `Copy`: its field `b` holds a `Box<int>`, which isn't
