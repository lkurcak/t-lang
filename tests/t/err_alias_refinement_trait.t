// A trait's method can't have refinements yet, written or through an
// alias.
type Square = { s: u8 | s < 64 };

trait Board {
    fn at(&self, s: Square) -> u8;
}

fn main() {}
// error: refinements can't be in a trait's methods yet
