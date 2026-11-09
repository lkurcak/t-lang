// Two fields are two places: a `&mut` to each is fine at once.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut t: (u8, u8) = (1, 2);
    let a = &mut t.0;
    let b = &mut t.1;
    *a = 10;
    *b = 20;
    out.line(&(t.0 + t.1)) // expect: 30
}
