fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 10u8;
    out.line(&f"{n:8}") // error: say which side of the width `n` goes: `{n:>8}`, `{n:<8}` or `{n:^8}`
}
