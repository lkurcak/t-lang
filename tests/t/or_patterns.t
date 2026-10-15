// Or-patterns: an arm matches if any alternative does.

enum Shape { Circle(int), Square(int), Rect(int, int), Empty }
enum Opt { None, Some(int) }

// Every alternative binds `side`, at the same type.
fn first_side(s: Shape) -> int {
    match s {
        Shape::Circle(side) | Shape::Square(side) | Shape::Rect(side, _) => side,
        Shape::Empty => 0,
    }
}

// Nested, and covering a whole type between them.
fn small(o: Opt) -> bool {
    match o {
        Opt::Some(0 | 1 | 2) => true,
        Opt::Some(_) | Opt::None => false,
    }
}

// Two or-patterns in different fields: every combination is reachable, and
// each alternative is used by some combination.
struct Light { on: bool, color: Color }
enum Color { Red, Green, Blue }

fn code(l: Light) -> int {
    match l {
        Light { on: true, color: Color::Red | Color::Green } => 1,
        Light { on: true | false, color: Color::Blue | Color::Red } => 2,
        Light { on: false, color: Color::Green } => 3,
    }
}

fn xor(a: bool, b: bool) -> bool {
    match a {
        true | false => a != b,
    }
}

// Alternatives may bind in a different order.
enum Pair { IntBool(int, bool), BoolInt(bool, int) }
struct S { a: int, b: int }

fn main() {
    print(first_side(Shape::Circle(1)));    // expect: 1
    print(first_side(Shape::Square(2)));    // expect: 2
    print(first_side(Shape::Rect(3, 9)));   // expect: 3
    print(first_side(Shape::Empty));        // expect: 0
    print_bool(small(Opt::Some(2)));        // expect: true
    print_bool(small(Opt::Some(3)));        // expect: false
    print_bool(small(Opt::None));           // expect: false
    print_bool(xor(true, false));           // expect: true
    print(code(Light { on: true, color: Color::Red }));     // expect: 1
    print(code(Light { on: false, color: Color::Red }));    // expect: 2
    print(code(Light { on: true, color: Color::Blue }));    // expect: 2
    print(code(Light { on: false, color: Color::Green }));  // expect: 3
    // Irrefutable, so fine in `let`.
    let Pair::IntBool(n, b) | Pair::BoolInt(b, n) = Pair::BoolInt(true, 7);
    print(n);                               // expect: 7
    print_bool(b);                          // expect: true
    let Pair::IntBool(m, c) | Pair::BoolInt(c, m) = Pair::IntBool(8, false);
    print(m);                               // expect: 8
    print_bool(c);                          // expect: false
    // The first alternative binds `x` = 0, then fails on `b: 0`. The second
    // then matches and binds `x` = 5.
    let x = match (S { a: 0, b: 5 }) {
        S { a: x, b: 0 } | S { a: 0, b: x } => x,
        _ => -1,
    };
    print(x);                               // expect: 5
}
