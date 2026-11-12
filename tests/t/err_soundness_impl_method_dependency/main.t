// A program's impl of a dependency's trait can't say more of an associated
// effect than the trait's method does, either: the dependency's generic
// code calls it going by the trait.
#[derive(Discard)]
struct Boom { n: u8 }

impl Iterator for Boom {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic("boom") }
}

#[derive(Discard)]
struct R {}

impl q::Run for R {
    fn run<I: Iterator<Item = u8, e = {}> + Discard>(mut i: I) -> u8 with {} {
        match i.next() {
            Some(v) => v,
            None => 0,
        }
    }
}

fn via<X: q::Run>() -> u8 with panic { X::run(Boom { n: 0 }) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: method `run` doesn't match its declaration in trait `q::Run`: its bound says `I: Iterator<e = {}>`, and the trait's doesn't
