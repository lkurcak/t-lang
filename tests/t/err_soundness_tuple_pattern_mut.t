// Through a `&mut` match ergonomics makes into a tuple's option, 99 is
// written where `square`'s result had a number below 64. `t` declares no
// refinement, so the write is allowed, but what was known of `t` is then
// forgotten: the match after can't take its `Some` to be below 64.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut t = (square(1), 2u8);
    match &mut t {
        (Some(a), _) => *a = 99,
        _ => {},
    }
    match t {
        (Some(x), _) => out.line(&need(x))?,
        _ => {},
    }
    Ok(())
}
// error: can't prove `s < 64` for argument `s` of `need`
