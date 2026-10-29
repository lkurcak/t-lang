// An effect parameter in a recursive group: `run` calls itself with `boom`
// for its `g`, so whatever `e` is, it can panic as well. The least effects
// are `{e, panic}`.
fn boom() { panic("boom") }

fn quiet() {}

fn run<effect e>(g: fn() with e, n: u8) {
    if n == 0 { g() } else { run(boom, n - 1) }
}

export fn safe() -> i64 with panic {
    run(quiet, 0);
    1
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&safe())?; // expect: 1
    Result::Ok(())
}
