mod a {
    pub fn f() -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    use a::f;
    out.line(&f())?;
    Result::Ok(())
}
// error: a `use` can't be in a function
