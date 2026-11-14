// `_` binds nothing: a function or a closure can have as many as it likes.
fn k(_: u8, _: u8) -> u8 { 3 }

fn apply<F: Fn(u8, u8) -> u8 + Discard>(f: F) -> u8 { f(1, 2) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&k(1, 2))?; // expect: 3
    out.line(&apply(|_, _| 4))?; // expect: 4
    out.line(&apply(|_, b| b))?; // expect: 2
    Ok(())
}
