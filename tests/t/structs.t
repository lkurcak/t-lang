// Structs: product types. A value has every field.

#[derive(Discard, Copy, Eq)]
struct Point { x: i64, y: i64 }
#[derive(Discard)]
struct Rect { min: Point, max: Point }

fn area(r: Rect) -> i64 with panic {
    (r.max.x - r.min.x) * (r.max.y - r.min.y)
}

fn translate(p: Point, dx: i64) -> Point with panic {
    Point { x: p.x + dx, y: p.y }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = Point { y: 2, x: 1 };       // field order doesn't matter
    out.line(&p.x)?;                    // expect: 1
    let r = Rect { min: p, max: Point { x: 4, y: 6 } };
    out.line(&area(r))?;                // expect: 12
    let x = 10;
    let y = 20;
    let q = Point { x, y };             // short for `Point { x: x, y: y }`
    out.line(&q.y)?;                    // expect: 20
    out.line(&(p == Point { x: 1, y: 2 }))?;  // expect: true
    out.line(&(p != q))?;               // expect: true
    out.line(&translate(p, 5).x)        // expect: 6
}
