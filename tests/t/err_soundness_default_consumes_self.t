// Calling a method of `&self` doesn't use `self` up.
struct Fd { n: int }

trait Tr {
    fn get(&self) -> int;

    fn consume(self) -> int { self.get() }
}

impl Tr for Fd {
    fn get(&self) -> int { self.n }
}

fn main() {
    print(Fd { n: 1 }.consume());
}
// error: `self` is never used up, and its type, `Self`, isn't `Discard`
