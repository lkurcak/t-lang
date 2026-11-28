// A function that calls itself with its own closure passes its own `F::e`
// on, `{F::e}`, which is no bigger: no polymorphic recursion. Nor with a
// written effect parameter.
fn run<F: Fn(u8) -> u8 + Copy + Discard>(f: F, n: u8) -> u8 {
    if n == 0 { f(0) } else { run(f, n - 1) }
}

fn run_e<effect e, F: Fn(u8) -> u8 with e + Copy + Discard>(f: F, n: u8) -> u8 with e {
    if n == 0 { f(1) } else { run_e(f, n - 1) }
}

fn quiet() -> u8 with {} { run(|x| x, 3) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&quiet())?;             // expect: 0
    out.line(&run(|x| x + 200, 2))?; // expect: 200
    out.line(&run_e(|x| x + 1, 2))?; // expect: 2
    Ok(())
}
