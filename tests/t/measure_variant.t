// A measure can say which variant an enum's value is: `is_some`,
// `is_none`, `is_ok` and `is_err` are (docs/59). So `List::pop` says it
// gives a `Some` exactly when the list wasn't empty.
use alloc::list::List;

fn small(n: u8) -> { r: Option<u8> | r.is_some() == (n < 100) } {
    if n < 100 { Some(n) } else { None }
}

/// Moves every value of `from` onto `to`, last first: how many is known.
fn drain(from: &mut List<u8>, to: &mut List<u8>) with {alloc, panic}
ensures
    to.len() == old(to.len()) + old(from.len()),
{
    let n = from.len() + to.len();
    while let Some(v) = from.pop() invariant from.len() + to.len() == n {
        to.push(v);
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let r = small(4);
    let t: { t: bool | t } = r.is_some();
    let Some(_) = small(7) else { return Ok(()) };
    // That `small(200)` is a `None` is known.
    let none: { b: bool | b } = small(200).is_none();
    out.line(&none)?;                       // expect: true
    out.line(&t)?;                          // expect: true
    let o: Result<u8, u8> = Err(3);
    let e: { e: bool | e } = o.is_err() && !o.is_ok();
    out.line(&e)?;                          // expect: true
    let mut a: List<u8> = List::new();
    a.push(1);
    a.push(2);
    let mut b: List<u8> = List::new();
    drain(&mut a, &mut b);
    let n: { n: usize | n == 2 } = b.len();
    out.line(&n)?;                          // expect: 2
    out.line(&b[0])?;                       // expect: 2
    Ok(())
}
