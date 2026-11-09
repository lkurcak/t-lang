// C calls an export, so it's used even if nothing in T calls it.
extern fn on_reset() {}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line("no warning") // expect: no warning
}
