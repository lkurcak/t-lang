// Structs: product types. A value has every field.

#[derive(Discard, Copy, Eq)]
struct Point { x: int, y: int }
#[derive(Discard)]
struct Rect { min: Point, max: Point }

fn area(r: Rect) -> int with panic {
    (r.max.x - r.min.x) * (r.max.y - r.min.y)
}

fn translate(p: Point, dx: int) -> Point with panic {
    Point { x: p.x + dx, y: p.y }
}

fn main() -> Point with panic {
    let p = Point { y: 2, x: 1 };       // field order doesn't matter
    print(p.x);                         // expect: 1
    let r = Rect { min: p, max: Point { x: 4, y: 6 } };
    print(area(r));                     // expect: 12
    let x = 10;
    let y = 20;
    let q = Point { x, y };             // short for `Point { x: x, y: y }`
    print(q.y);                         // expect: 20
    print_bool(p == Point { x: 1, y: 2 });  // expect: true
    print_bool(p != q);                 // expect: true
    translate(p, 5)                     // expect: Point { x: 6, y: 2 }
}
