// Or-patterns: an arm matches if any alternative does.

#[derive(Discard)]
enum Shape { Circle(i64), Square(i64), Rect(i64, i64), Empty }
#[derive(Discard)]
enum Opt { None, Some(i64) }

// Every alternative binds `side`, at the same type.
fn first_side(s: Shape) -> i64 {
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
#[derive(Discard)]
struct Light { on: bool, color: Color }
#[derive(Discard)]
enum Color { Red, Green, Blue }

fn code(l: Light) -> i64 {
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
#[derive(Discard)]
enum Pair { IntBool(i64, bool), BoolInt(bool, i64) }
#[derive(Discard)]
struct S { a: i64, b: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&first_side(Shape::Circle(1)))?;    // expect: 1
    out.line(&first_side(Shape::Square(2)))?;    // expect: 2
    out.line(&first_side(Shape::Rect(3, 9)))?;   // expect: 3
    out.line(&first_side(Shape::Empty))?;        // expect: 0
    out.line(&small(Opt::Some(2)))?;        // expect: true
    out.line(&small(Opt::Some(3)))?;        // expect: false
    out.line(&small(Opt::None))?;           // expect: false
    out.line(&xor(true, false))?;           // expect: true
    out.line(&code(Light { on: true, color: Color::Red }))?;     // expect: 1
    out.line(&code(Light { on: false, color: Color::Red }))?;    // expect: 2
    out.line(&code(Light { on: true, color: Color::Blue }))?;    // expect: 2
    out.line(&code(Light { on: false, color: Color::Green }))?;  // expect: 3
    // Irrefutable, so fine in `let`.
    let Pair::IntBool(n, b) | Pair::BoolInt(b, n) = Pair::BoolInt(true, 7);
    out.line(&n)?;                               // expect: 7
    out.line(&b)?;                          // expect: true
    let Pair::IntBool(m, c) | Pair::BoolInt(c, m) = Pair::IntBool(8, false);
    out.line(&m)?;                               // expect: 8
    out.line(&c)?;                          // expect: false
    // The first alternative binds `x` = 0, then fails on `b: 0`. The second
    // then matches and binds `x` = 5.
    let x = match (S { a: 0, b: 5 }) {
        S { a: x, b: 0 } | S { a: 0, b: x } => x,
        _ => -1,
    };
    out.line(&x)?;                               // expect: 5
    Result::Ok(())
}
