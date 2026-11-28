// An opaque result's `Fn` bound with no `with` leaves what its calls do
// open too: a caller whose effects are inferred has them. `with {}` says
// they do nothing, and `with F::e` passes on an open bound's.
fn adder(n: i64) -> impl Fn(i64) -> i64 + Discard { move |x| x + n }

fn quiet(n: i64) -> impl Fn(i64) -> i64 with {} + Discard { move |x| if x < n { n } else { x } }

fn wrap<F: Fn(i64) -> i64 + Discard>(f: F) -> impl Fn(i64) -> i64 with F::e + Discard {
    move |x| f(x)
}

fn none() -> i64 with {} {
    let q = quiet(3);
    let w = wrap(|x| if x < 0 { 0 } else { x });
    if q(1) > w(0) { 0 } else { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = adder(2);
    out.line(&a(1))?;   // expect: 3
    out.line(&none())?; // expect: 0
    let w = wrap(|x| x * 2);
    out.line(&w(21))?; // expect: 42
    Ok(())
}
