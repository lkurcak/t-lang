// `P { mut x }` binding `x`, which nothing reads: the fix spells the
// shorthand out, `P { x: mut _x }`.
#[derive(Discard)]
struct P { x: i64, y: i64 }

#[allow(unused_mut)]
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let P { mut x, y } = P { x: 1, y: 2 };
    out.line(&y) // expect: 2
}
// warning: unused variable `x`
