// A const in an alternative of a `let`'s or-pattern, which always
// matches; and a `&str` const matched through a reference to a `&str`.
const C: u8 = 3;
const KW: &str = "fn";

fn word(s: &&str) -> u8 {
    match s {
        KW => 1,
        _ => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (C, x) | (_, x) = (3u8, 4u8);
    out.line(&x)?; // expect: 4
    let (C, y) | (_, y) = (5u8, 6u8);
    out.line(&y)?; // expect: 6
    out.line(&word(&"fn"))?; // expect: 1
    out.line(&word(&"let"))?; // expect: 0
    Ok(())
}
