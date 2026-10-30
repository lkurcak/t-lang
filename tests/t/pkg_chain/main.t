// A program can depend on a package and on one that depends on it: both
// name the one package, so `shapes::Point` is `geometry::Point`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p: geometry::Point = shapes::origin();
    let q = shapes::Point { x: p.x + 1, y: 2 };
    out.line(&(q.x + q.y))?; // expect: 3
    Result::Ok(())
}
