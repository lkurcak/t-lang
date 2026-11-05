// An operator in a hole that may overflow panics, as anywhere: a
// function that says no `panic` must prove it doesn't.
fn show(n: i64, out: &mut std::io::Stdout) -> Result<(), std::io::Error> with {} {
    out.line(&f"{n + 1}")
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    show(1, &mut out)
}
// error: `+` may panic
