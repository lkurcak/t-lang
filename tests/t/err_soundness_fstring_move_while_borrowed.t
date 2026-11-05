// While a format string that names `s` is still used, `s` can't move.
use alloc::string::String;

fn take(s: String) -> usize { s.len() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::of(&"hi");
    let line = f"<{s}>";
    let n = take(s);
    out.line(&line)?;
    out.line(&n)?;
    Result::Ok(())
}
// error: cannot move out of `s` while `s` is borrowed
