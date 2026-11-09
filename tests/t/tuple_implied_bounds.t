// A parameter's type is well-formed, which the function may count on: a
// `&'a &'b u8` in a tuple means `'b` outlives `'a`, so `*x.0` is a
// `&'a u8`, though the signature doesn't say `'b: 'a`.
fn inner<'a, 'b>(x: (&'a &'b u8, u8)) -> &'a u8 { *x.0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let v: u8 = 5;
    let r = &v;
    out.line(inner((&r, 0)))  // expect: 5
}
