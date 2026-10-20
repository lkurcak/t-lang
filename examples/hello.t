#[derive(Discard)]
enum Shape { Circle(int), Rect(int, int), Empty }

fn area(s: Shape) -> int with panic {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn fact(n: int) -> int with panic {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn twice<e, F: Fn(int) -> int with e + Discard>(f: F, x: int) -> int with e { f(f(x)) }

fn main() with panic {
    let n = 5;
    print(fact(10));                                    // expect: 3628800
    print(twice(fact, 3));                              // expect: 720
    print(twice(|x| x + n, 0));                         // expect: 10
    print(area(Shape::Rect(3, 4)));                     // expect: 12
    print(area(Shape::Circle(2)) + area(Shape::Empty)); // expect: 12
}
