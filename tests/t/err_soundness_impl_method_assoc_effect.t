// An impl's method says its iterator's `next` does nothing, `e = {}`, and
// so says `with {}` itself; its trait's says nothing of `e`. A call through
// the trait gives it an iterator whose `next` panics, in a method that
// says it can't (retrospective 8, since 0.83.0).
#[derive(Discard)]
struct Boom { n: u8 }

impl Iterator for Boom {
    type Item = u8;
    effect e = panic;

    fn next(&mut self) -> Option<u8> with panic { panic("boom") }
}

trait Run {
    fn run<I: Iterator<Item = u8> + Discard>(i: I) -> u8 with I::e;
}

#[derive(Discard)]
struct R {}

impl Run for R {
    fn run<I: Iterator<Item = u8, e = {}> + Discard>(mut i: I) -> u8 with {} {
        match i.next() {
            Some(v) => v,
            None => 0,
        }
    }
}

fn via<X: Run>() -> u8 with panic { X::run(Boom { n: 0 }) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: its bound says `I: Iterator<e = {}>`, and the trait's doesn't
