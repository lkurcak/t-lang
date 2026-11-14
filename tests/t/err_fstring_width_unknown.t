fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 10u8;
    out.line(&f"{n:>w$}") // error: unknown variable `w`
}
