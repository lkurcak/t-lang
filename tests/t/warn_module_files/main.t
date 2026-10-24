// Warnings are in the file they're about, and fixes change every file that
// needs it: renaming `Helper` renames it in `util.t` and here.
mod util;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&util::Helper())?;   // expect: 2
    Result::Ok(())
}
