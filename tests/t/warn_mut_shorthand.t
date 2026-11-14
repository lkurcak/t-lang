// `P { mut x }` with `x` never changed: the fix removes the `mut`, leaving `P { x }`.

#[derive(Discard)]
struct P { x: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let P { mut x } = P { x: 1 };
    out.line(&x) // expect: 1
}
// warning: variable `x` doesn't need to be mutable
