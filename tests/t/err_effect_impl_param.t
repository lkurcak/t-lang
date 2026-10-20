// An effect parameter of an impl must be decided by an `Fn` bound, if it
// isn't in the impl's type.
trait Tr {
    fn a(&self) -> int;
}

#[derive(Discard)]
struct W {}

impl<f> Tr for W {
    fn a(&self) -> int { 1 }
}

fn main() {}
// error: effect parameter `f` isn't decided by anything: say it's the effects of something the impl's type holds, with a bound like `F: Fn() with f`
