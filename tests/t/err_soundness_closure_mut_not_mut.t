// Calling a closure that changes what it captured borrows it mutably.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n: i64 = 0;
    let inc = || n = n + 1;
    inc();
    out.line(&n)?;
    Result::Ok(())
}
// error: cannot borrow `inc` as mutable, as `inc` isn't declared `mut`
