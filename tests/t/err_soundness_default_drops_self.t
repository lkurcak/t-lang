// A default method is checked once, for every `Self`, so it can't throw
// `self` away unless `Discard` is a supertrait.
struct Fd { n: i64 }

trait Eat {
    fn eat(self) -> i64 { 0 }
}

impl Eat for Fd {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(Fd { n: 1 }.eat()))?;
    Result::Ok(())
}
// error: `self` is never used up, and its type, `Self`, isn't `Discard`; to let it be thrown away, make `Discard` a supertrait of the trait
