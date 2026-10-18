#[derive(Discard, Copy)]
enum E<'a> { A(&'a mut int), B }

fn main() {}
// error: `E` can't be `Copy`: its variant `A` holds a `&'a mut int`, which isn't
