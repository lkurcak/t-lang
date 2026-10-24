// An impl gives each associated item as what the trait declares it as:
// `e` is an associated effect, so `type e = i64;` doesn't give it.
trait Tr {
    effect e;
    fn run(&self) with Self::e;
}

#[derive(Discard)]
struct W {}

impl Tr for W {
    type e = i64;

    fn run(&self) {}
}

fn main() {}
// error: `e` isn't an associated type of trait `Tr`
