// An impl for a type that doesn't meet a method's `where` clause still
// defines the method, assuming the clause: nothing can call it, as no call
// can prove the clause.
struct Fd { n: int }

trait Tr {
    fn gone(self) -> int where Self: Discard;

    fn keep(self) -> Self;
}

impl Tr for Fd {
    // Throws `self` away, which it could if it were `Discard`.
    fn gone(self) -> int where Self: Discard { 0 }

    fn keep(self) -> Fd { self }
}

fn main() {
    let Fd { n } = Fd { n: 1 }.keep();
    print(n); // expect: 1
}
