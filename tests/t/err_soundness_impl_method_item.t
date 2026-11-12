// An impl's method bounds its `I` more than its trait's does: it counts on
// `I`'s items being `u8`s. A call through the trait, which shows only what
// the trait's bound says, would give it `Chars`, whose items are `char`s,
// and the method would read each as a `u8` (retrospective 8, since 0.7.0).
trait Run {
    fn run<I: Iterator + Discard>(i: I) -> u8 with I::e;
}

#[derive(Discard)]
struct R {}

impl Run for R {
    fn run<I: Iterator<Item = u8> + Discard>(mut i: I) -> u8 with I::e {
        match i.next() {
            Some(v) => v,
            None => 0,
        }
    }
}

fn via<X: Run>() -> u8 { X::run("ab".chars()) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: method `run` doesn't match its declaration in trait `Run`: its bound says `I: Iterator<Item = u8>`, and the trait's doesn't: a call through the trait wouldn't show it holds
