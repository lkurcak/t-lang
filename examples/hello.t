#[derive(Discard)]
enum Shape { Circle(i64), Rect(i64, i64), Empty }

fn area(s: Shape) -> i64 with panic {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn fact(n: i64) -> i64 with panic {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn twice<effect e, F: Fn(i64) -> i64 with e + Discard>(f: F, x: i64) -> i64 with e { f(f(x)) }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let n = 5;
    out.line(&fact(10))?;                                    // expect: 3628800
    out.line(&twice(fact, 3))?;                              // expect: 720
    out.line(&twice(|x| x + n, 0))?;                         // expect: 10
    out.line(&area(Shape::Rect(3, 4)))?;                     // expect: 12
    out.line(&(area(Shape::Circle(2)) + area(Shape::Empty)))?; // expect: 12
    Result::Ok(())
}
