// The other way is fine: a tuple of longer-lived references is one of
// shorter-lived ones. And a reference into a tuple of references is good
// for as long as both it and what they point to are.
fn shorter<'a>(t: (&'static str, u8), _with: &'a str) -> (&'a str, u8) { t }

fn first<'a, 'b>(x: &'a (&'b u8, u8)) -> &'a u8 { x.0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = "local";
    let (a, n) = shorter(("static", 1), s);
    out.line(a)?;           // expect: static
    out.line(&n)?;          // expect: 1
    let v: u8 = 7;
    let pair = (&v, 2u8);
    out.line(first(&pair))  // expect: 7
}
