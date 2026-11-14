fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (n, w) = (10u8, 4u32);
    out.line(&f"{n:>w$}") // error: expected `usize`, found `u32`
}
