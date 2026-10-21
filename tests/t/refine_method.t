// An inherent method can have refinements too.
#[derive(Copy, Discard)]
struct Meter { n: int }

impl Meter {
    fn split(&self, parts: { p: int | p > 0 }) -> int { self.n / parts }
}

fn main() {
    let m = Meter { n: 12 };
    print(m.split(4)); // expect: 3
}
