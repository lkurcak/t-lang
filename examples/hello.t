enum Shape {
    Circle(int),
    Rect(int, int),
    Empty,
}

fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Empty => 0,
    }
}

fn fact(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * fact(n - 1)
    }
}

fn twice(f: fn(int) -> int, x: int) -> int {
    f(f(x))
}

fn main() {
    let n = 5;
    print(fact(10));
    print(twice(fact, 3));
    print(twice(|x| x + n, 0));
    print(area(Shape::Rect(3, 4)));
    print(area(Shape::Circle(2)) + area(Shape::Empty));
}
