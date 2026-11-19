// A law of a method with a default body: the default is checked once, for
// every type of the law's trait, and an impl that writes its own is
// checked as any impl's (docs/88-iterator-laws.md).
trait Steps {
    fn step(&self) -> usize;

    fn twice(&self) -> usize with panic { self.step() + self.step() }
}

trait Small: Steps {
    law fn step(&self) -> { s: usize | s < 100 };

    law fn twice(&self) -> { t: usize | t < 200 } with panic;
}

#[derive(Discard)]
struct A {}

impl Steps for A {
    fn step(&self) -> usize { 5 }
}

impl Small for A {}

#[derive(Discard)]
struct B {}

impl Steps for B {
    fn step(&self) -> usize { 7 }

    fn twice(&self) -> usize { 14 }
}

impl Small for B {}

fn index<T: Small>(t: &T) -> u8 with panic {
    let a = [3u8; 200];
    a[t.twice()]
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&index(&A {}))?; // expect: 3
    out.line(&index(&B {}))?; // expect: 3
    Ok(())
}
