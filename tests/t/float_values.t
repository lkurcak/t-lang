// Floats in structs, enums, arrays and generic code: a value like any
// other, copied, swapped and written.
use core::mem::swap;

#[derive(Copy, Debug, Discard)]
struct Point { x: f64, y: f32 }

#[derive(Copy, Debug, Discard)]
enum Shape { Circle(f64), Square(f32) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Square(side) => (side * side) as f64,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut p = Point { x: 1.5, y: -0.25 };
    let mut q = Point { x: f64::nan(), y: 2.0 };
    swap(&mut p, &mut q);
    out.line(&f"{p:?} {q:?}")?; // expect: Point { x: NaN, y: 2.0 } Point { x: 1.5, y: -0.25 }
    out.line(&area(Shape::Circle(2.0)))?; // expect: 12.0
    out.line(&area(Shape::Square(1.5)))?; // expect: 2.25
    let xs: [f32; 3] = [1.0, 2.0, 3.0];
    out.line(&f"{xs:?} {Some(0.5f64):?}")?; // expect: [1.0, 2.0, 3.0] Some(0.5)
    Ok(())
}
