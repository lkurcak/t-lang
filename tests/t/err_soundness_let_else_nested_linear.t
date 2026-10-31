// A `Some` of a square doesn't match, and the `else` can't name the `Fd`
// in it.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

enum Shape { Circle(Fd), Square(Fd) }

fn f(o: Option<Shape>) -> i64 {
    let Option::Some(Shape::Circle(fd)) = o else { return 0 };
    close(fd)
}

fn main() {
    let _ = f(Option::None);
}
// error: if the pattern of this `let .. else` doesn't match, it throws away `o.0`, and its type, `Shape`
