// `let` takes any irrefutable pattern: one that matches every value of its type.

#[derive(Discard)]
struct Point { x: i64, y: i64 }
#[derive(Discard)]
struct Line { from: Point, to: Point }
#[derive(Discard)]
enum Wrapper { Only(i64) }   // one variant, so matching it can't fail

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let Point { x, y } = Point { x: 1, y: 2 };
    out.line(&(x + y))?;           // expect: 3
    let Line { from: Point { x: x0, .. }, to: Point { y: y1, .. } } =
        Line { from: Point { x: 10, y: 0 }, to: Point { x: 0, y: 20 } };
    out.line(&(x0 + y1))?;         // expect: 30
    let Wrapper::Only(n) = Wrapper::Only(5);
    out.line(&n)?;               // expect: 5
    let _: i64 = 99;
    let () = ();
    Result::Ok(())
}
