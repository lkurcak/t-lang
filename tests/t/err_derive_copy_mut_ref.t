#[derive(Discard, Copy)]
enum E<'a> { A(&'a mut i64), B }

fn main() {}
// error: `E` can't be `Copy`: its variant `A` holds a `&'a mut i64`, which isn't
