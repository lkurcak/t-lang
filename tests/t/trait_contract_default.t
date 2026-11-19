// A trait's method with a body, a default, is checked once, with what the
// trait says of it; an impl that has its own is checked with that too
// (docs/86-trait-contracts.md).
trait Halve {
    fn get(&self) -> u32;

    /// Half of `get`, which is no more than it.
    fn half(&self) -> { h: u32 | h <= 1000 } {
        let g = self.get();
        if g > 2000 { 1000 } else { g / 2 }
    }
}

#[derive(Discard)]
struct A { n: u32 }

impl Halve for A {
    fn get(&self) -> u32 { self.n }
}

#[derive(Discard)]
struct B {}

impl Halve for B {
    fn get(&self) -> u32 { 7 }

    fn half(&self) -> u32 { 3 }
}

fn pair<T: Halve, U: Halve>(t: &T, u: &U) -> { s: u32 | s <= 2000 } { t.half() + u.half() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pair(&A { n: 10 }, &B {}))?; // expect: 8
    out.line(&A { n: 5000 }.half())?;    // expect: 1000
    Ok(())
}
