// An impl for a type that doesn't meet a method's `where` clause still
// defines the method, assuming the clause: nothing can call it, as no call
// can prove the clause.
struct Fd { n: i64 }

trait Tr {
    fn gone(self) -> i64 where Self: Discard;

    fn keep(self) -> Self;
}

impl Tr for Fd {
    // Throws `self` away, which it could if it were `Discard`.
    fn gone(self) -> i64 where Self: Discard { 0 }

    fn keep(self) -> Fd { self }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let Fd { n } = Fd { n: 1 }.keep();
    out.line(&n)?; // expect: 1
    Result::Ok(())
}
