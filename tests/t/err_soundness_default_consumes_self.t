// Calling a method of `&self` doesn't use `self` up.
struct Fd { n: i64 }

trait Tr {
    fn get(&self) -> i64;

    fn consume(self) -> i64 { self.get() }
}

impl Tr for Fd {
    fn get(&self) -> i64 { self.n }
}

fn main() {
    print(Fd { n: 1 }.consume());
}
// error: `self` is never used up, and its type, `Self`, isn't `Discard`
