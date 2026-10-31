// `if let` runs its block when the pattern matches, with what it binds;
// `else if let` and `else` try what comes next.
#[derive(Discard, Copy)]
enum Shape { Circle(i64), Rect(i64, i64), Empty }

fn area(s: Shape) -> i64 {
    if let Shape::Circle(r) = s {
        3 * r * r
    } else if let Shape::Rect(w, h) = s {
        w * h
    } else {
        0
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&area(Shape::Circle(2)))?; // expect: 12
    out.line(&area(Shape::Rect(2, 5)))?; // expect: 10
    out.line(&area(Shape::Empty))?; // expect: 0
    let mut n = 0;
    if let Option::Some(k) = Option::Some(4) {
        n = k;
    }
    if let Option::Some(k) = Option::<i64>::None {
        n = k + 100;
    }
    out.line(&n)?; // expect: 4
    // A pattern that looks inside: a `Some` of something else goes to the
    // `else`.
    let s = Option::Some(Shape::Rect(1, 1));
    if let Option::Some(Shape::Circle(r)) = s {
        out.line(&r)?;
    } else {
        out.line(&7i64)?; // expect: 7
    }
    // Through a reference, it only looks.
    let o = Option::Some(5i64);
    if let Option::Some(k) = &o {
        out.line(k)?; // expect: 5
    }
    if let Option::Some(k) = o {
        out.line(&k)?; // expect: 5
    }
    Result::Ok(())
}
