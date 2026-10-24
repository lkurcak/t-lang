// A part of a variable with no value can't be given one: the whole
// hasn't got one to be part of.
#[derive(Discard)]
struct P { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p: P;
    p.x = 1;
    p.y = 2;
    out.line(&p.x)?;
    Result::Ok(())
}
// error: cannot assign to `p.x`, part of `p`, which has no value here
