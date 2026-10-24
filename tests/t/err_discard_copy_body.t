// A `Copy` value is copied and dropped where nothing in the program says
// so, so it can't have a `discard` of its own: dropping one is nothing.
#[derive(Copy)]
struct Id { n: i64 }

impl Discard for Id {
    fn discard(self) {}
}

fn main() {}
// error: `Id` is `Copy`, so it can't have a `discard` of its own
