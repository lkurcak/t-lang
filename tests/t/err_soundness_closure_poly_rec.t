// Each call wraps `g` in a new closure, whose type holds `G`: the copies
// monomorphisation would make never end.
fn f<effect e, G: Fn() -> i64 with e + Discard>(g: G, n: i64) -> i64 with {e, panic} {
    if n == 0 { g() } else { f(|| g() + 1, n - 1) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(|| 0, 3))?;
    Result::Ok(())
}
// error: polymorphic recursion
