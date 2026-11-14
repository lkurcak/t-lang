fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 10u8;
    out.line(&f"{n:4x}") // error: say which side of the width `n` goes: `{n:>4x}`, `{n:<4x}` or `{n:^4x}`
}
