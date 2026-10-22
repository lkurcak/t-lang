// An inherent method can have refinements too.
#[derive(Copy, Discard)]
struct Meter { n: i64 }

impl Meter {
    fn split(&self, parts: { p: i64 | p > 0 }) -> i64 { self.n / parts }
}

fn main() {
    let m = Meter { n: 12 };
    print(m.split(4)); // expect: 3
}
