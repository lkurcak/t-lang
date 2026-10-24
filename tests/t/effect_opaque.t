// An opaque type's bound can have effects, and pass on a parameter's.
fn adder(n: i64) -> impl Fn(i64) -> i64 with panic + Discard { move |x| x + n }

fn wrap<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F) -> impl Fn(i64) -> i64 with e + Discard { move |x| f(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let a = adder(2);
    out.line(&a(1))?; // expect: 3
    let w = wrap(|x| if x < 0 { 0 } else { x });
    out.line(&w(5))?; // expect: 5
    Result::Ok(())
}
