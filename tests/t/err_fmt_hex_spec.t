fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 10u8;
    out.line(&f"{n:X}") // error: a hole in a format string can say `:?`, for `Debug`, or `:x` or `:08x`
}
