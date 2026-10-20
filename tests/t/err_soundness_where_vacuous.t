// An impl for a linear type may define a method whose `where` clause it
// doesn't meet; nothing can call it.
struct Fd { n: int }

trait Tr {
    fn gone(self) -> int where Self: Discard;
}

impl Tr for Fd {
    fn gone(self) -> int where Self: Discard { 0 }
}

fn main() {
    print(Fd { n: 1 }.gone());
}
// error: `gone` needs its `Self` to implement `Discard`, and `Fd` doesn't
