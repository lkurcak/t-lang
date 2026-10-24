// A trait's provided method is checked against the trait, where all that's
// known of `next` is `Self::e`.
trait Gen {
    effect e;

    fn next(&mut self) -> i64 with Self::e;

    fn two(&mut self) -> i64 { self.next() }
}

#[derive(Discard)]
struct Z {}

impl Gen for Z {
    effect e = panic;

    fn next(&mut self) -> i64 with panic { panic("stop") }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut z = Z {};
    out.line(&z.two())?;
    Result::Ok(())
}
// error: `next` may have the effects `Self::e`, but `two` doesn't say `with Self::e`
