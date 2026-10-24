// Ranges of characters that touch can be merged, and the fix writes
// characters.
fn f(c: char) -> bool {
    match c {
        'a'..='m' | 'n'..='z' => true,
        _ => false,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f('q'))?; // expect: true
    Result::Ok(())
}
// warning: `'a'..='m'` and `'n'..='z'` can be merged into `'a'..='z'`
