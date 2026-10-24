// `'a'` is a character, `'a` a lifetime.
fn first<'a>(c: &'a char, d: &'a char) -> &'a char { if *c == 'a' { c } else { d } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 'a';
    let y = 'b';
    out.line(&(*first(&x, &y) as u32))?; // expect: 97
    out.line(&(*first(&y, &x) as u32))?; // expect: 97
    Result::Ok(())
}
