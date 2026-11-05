// A const can be any plain data: structs, enums and arrays of integers,
// bools, chars, strings. Its type needn't be `Copy`: each use is a fresh
// value, as if a function made it.
#[derive(Discard)]
struct Point { x: i64, y: i64 }

#[derive(Discard)]
enum Shape { Dot(Point), Line(Point, Point) }

const ORIGIN: Point = Point { x: 0, y: 0 };
const UNIT: Shape = Shape::Line(ORIGIN, Point { x: 1, y: 1 });
const DOT: Shape = Shape::Dot(Point { x: 5, y: 5 });
const PRIMES: [u8; 4] = [2, 3, 5, 7];
const NAMES: [&str; 2] = ["ada", "alan"];
const MAYBE: Option<u8> = Some(7);

fn squares() -> [u32; 4] {
    let mut a = [0u32; 4];
    let mut i: usize = 0;
    while i < 4 {
        let n = i as u32;
        a[i] = n * n;
        i = i + 1;
    }
    a
}

// Computed by a loop, once, while compiling.
const SQUARES: [u32; 4] = squares();

fn len(s: Shape) -> i64 {
    match s {
        Shape::Dot(_) => 0,
        Shape::Line(a, b) => (b.x - a.x) + (b.y - a.y),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a = ORIGIN;
    let b = ORIGIN; // a second one, not a move of the first
    out.line(&(a.x + b.y))?;        // expect: 0
    out.line(&len(UNIT))?;          // expect: 2
    out.line(&len(UNIT))?;          // expect: 2
    out.line(&len(DOT))?;           // expect: 0
    out.line(&PRIMES[3])?;          // expect: 7
    out.line(&NAMES[1])?;           // expect: alan
    out.line(&MAYBE.unwrap_or(0))?; // expect: 7
    out.line(&SQUARES[3])?;         // expect: 9
    Ok(())
}
