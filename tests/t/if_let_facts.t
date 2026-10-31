// What a pattern says of an integer holds in the block it guards, as in a
// `match` arm, and after a `let .. else`.
fn digit(n: u8) -> { d: u8 | d <= 9 } {
    if let 0..=9 = n { n } else { 0 }
}

fn small(n: u8) -> { d: u8 | d < 10 } {
    let 0..=9 = n else { return 0 };
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&digit(7))?; // expect: 7
    out.line(&digit(70))?; // expect: 0
    out.line(&small(3))?; // expect: 3
    Result::Ok(())
}
