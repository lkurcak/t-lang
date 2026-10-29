// Results borrowed through nested closures keep their own inferred
// lifetimes: from a capture, or from the outer closure's parameter.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 5;
    let get = || { let inner = || &n; inner() };
    out.line(&(*get()))?; // expect: 5

    let id = |x: &i64| { let inner = || x; inner() };
    let long = id(&n);
    {
        let short: i64 = 7;
        out.line(&(*id(&short)))?; // expect: 7
    }
    out.line(&(*long))?; // expect: 5
    Result::Ok(())
}
