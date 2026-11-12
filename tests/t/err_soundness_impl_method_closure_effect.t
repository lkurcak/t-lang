// An impl's method takes only closures that do nothing, `with {}`, where
// its trait's takes any, `with g`. A call through the trait gives it one
// that panics (retrospective 8, since 0.8.0).
trait Run {
    fn run<effect g, F: FnMut() -> u8 with g + Discard>(f: F) -> u8 with g;
}

#[derive(Discard)]
struct R {}

impl Run for R {
    fn run<effect g, F: FnMut() -> u8 with {} + Discard>(mut f: F) -> u8 with {} { f() }
}

fn boom() -> u8 with panic { panic("boom") }

fn via<X: Run>() -> u8 with panic { X::run(boom) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&via::<R>())?;
    Ok(())
}
// error: its bound says `F: FnMut() -> u8`, where the trait's says `F: FnMut() -> u8 with g`
