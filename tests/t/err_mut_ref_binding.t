#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let o = Option::Some(1);
    match &o {
        Option::Some(mut n) => out.line(&(*n))?,
        Option::None => {}
    }
    Result::Ok(())
}
// error: `n` can't be `mut` here: it's bound by reference
