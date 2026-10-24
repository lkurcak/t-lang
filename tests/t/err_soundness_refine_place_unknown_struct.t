// A struct a call returns: nothing's known of its fields but its
// refinements.
#[derive(Copy, Discard)]
struct P { x: i64 }

fn make() -> P { P { x: 0 } }

fn f() -> i64 { 10 / make().x }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f())?;
    Result::Ok(())
}
// error: `/` may panic, but `f` doesn't say `with panic`
