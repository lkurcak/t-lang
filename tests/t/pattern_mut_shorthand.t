// `S { a, mut b }` binds field `b` to a `mut` variable `b`: the same as
// `b: mut b`, as `S { a }` is `S { a: a }`.
#[derive(Discard)]
struct P { x: i64, y: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let P { x, mut y } = P { x: 1, y: 2 };
    y = y + x;
    out.line(&y)?; // expect: 3
    let P { mut x, .. } = P { x: 5, y: 0 };
    x = x * 2;
    out.line(&x) // expect: 10
}
