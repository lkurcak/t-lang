// A trait's provided method is checked against the trait, where all that's
// known of `next` is `Self::e`.
trait Gen {
    effect e;

    fn next(&mut self) -> int with Self::e;

    fn two(&mut self) -> int { self.next() }
}

#[derive(Discard)]
struct Z {}

impl Gen for Z {
    effect e = panic;

    fn next(&mut self) -> int with panic { panic() }
}

fn main() {
    let mut z = Z {};
    print(z.two());
}
// error: `next` may have the effects `Self::e`, but `two` doesn't say `with Self::e`
