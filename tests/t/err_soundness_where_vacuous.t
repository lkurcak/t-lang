// An impl for a linear type may define a method whose `where` clause it
// doesn't meet; nothing can call it.
struct Fd { n: i64 }

trait Tr {
    fn gone(self) -> i64 where Self: Discard;
}

impl Tr for Fd {
    fn gone(self) -> i64 where Self: Discard { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Fd { n: 1 }.gone()))?;
    Result::Ok(())
}
// error: `gone` needs its `Self` to implement `Discard`, and `Fd` doesn't
