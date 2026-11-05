// `copied` turns an `Option<&T>` into an `Option<T>`, for a `Copy` `T`:
// what `match o { Some(x) => *x, None => d }` spells out.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let xs = [10u8, 20, 30];
    let s = xs.as_slice();
    out.line(&s.get(1).copied().unwrap_or(0))?; // expect: 20
    out.line(&s.get(7).copied().unwrap_or(0))?; // expect: 0
    let x: Option<u8> = s.first().copied();
    out.line(&(x == Option::Some(10)))?; // expect: true
    Result::Ok(())
}
