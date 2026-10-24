// `///` documents the `fn`, `struct`, `enum`, field or variant after it.
// Editors show it on hover. It changes nothing about what the program does.

/// A pair of ints.
#[derive(Discard)]
struct Pair {
    /// The first.
    a: i64,
    /// The second.
    ///
    /// Doc comments can run over several lines.
    b: i64,
}

/// A shape.
#[derive(Discard)]
enum Shape {
    /// Its radius.
    Circle(i64),
    /// No shape at all.
    Empty,
}

/// The sum of both halves of `p`.
// A plain comment between a doc comment and its function is fine.
fn sum(p: Pair) -> i64 with panic { p.a + p.b }

/// Roughly, with π rounded down.
fn area(s: Shape) -> i64 with panic {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Empty => 0,
    }
}

//// Four slashes make a plain comment, as in Rust.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&sum(Pair { a: 1, b: 2 }))?;                    // expect: 3
    out.line(&(area(Shape::Circle(2)) + area(Shape::Empty)))?; // expect: 12
    Result::Ok(())
}
