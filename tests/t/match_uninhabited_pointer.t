// A reference always points to a value, and a box always holds one, so a
// `&Never` or `Box<Never>` can't exist, and nor can a value that must hold
// itself forever. A `match` needn't cover them.

#[derive(Discard)]
enum Never {}

#[allow(infinite_types)]
#[derive(Discard)]
struct Stream { next: Box<Stream> }

#[derive(Discard)]
enum Shape<'a> { Circle(i64), Ref(&'a Never), Boxed(Box<Never>), Forever(Stream) }

fn radius(s: Shape) -> i64 {
    match s {
        Shape::Circle(r) => r,
    }
}

fn main() {
    print(radius(Shape::Circle(2))); // expect: 2
}
// warning: variant `Shape::Ref` is never constructed
// warning: variant `Shape::Boxed` is never constructed
// warning: variant `Shape::Forever` is never constructed
