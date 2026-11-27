// A float isn't `Eq`: its `==` isn't an equivalence, as NaN isn't equal to
// itself, which code over an `Eq` may count on.
fn same<T: Eq>(a: &T) -> bool { a == a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&same(&f64::nan()))?;
    Ok(())
}
// error: `same` needs its `T` to implement `Eq`, and `f64` doesn't
