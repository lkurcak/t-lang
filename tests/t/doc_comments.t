// `///` documents the `fn`, `struct`, `enum`, field or variant after it.
// Editors show it on hover. It changes nothing about what the program does.

/// A pair of ints.
struct Pair {
    /// The first.
    a: int,
    /// The second.
    ///
    /// Doc comments can run over several lines.
    b: int,
}

/// A shape.
enum Shape {
    /// Its radius.
    Circle(int),
    /// No shape at all.
    Empty,
}

/// The sum of both halves of `p`.
// A plain comment between a doc comment and its function is fine.
fn sum(p: Pair) -> int { p.a + p.b }

/// Roughly, with π rounded down.
fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Empty => 0,
    }
}

//// Four slashes make a plain comment, as in Rust.
fn main() {
    print(sum(Pair { a: 1, b: 2 }));                    // expect: 3
    print(area(Shape::Circle(2)) + area(Shape::Empty)); // expect: 12
}
