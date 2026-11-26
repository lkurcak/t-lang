// `..` in a variant's, a tuple's or an array's pattern stands for the
// values it doesn't name, as many `_`s as there are: none or more.
#[derive(Discard)]
enum Shape {
    Dot,
    Line(u8, u8),
    Box3(u8, u8, u8),
}

fn kind(s: &Shape) -> u8 {
    match s {
        Shape::Dot => 0,
        Shape::Line(..) => 1,
        Shape::Box3(..) => 2,
    }
}

fn first(s: &Shape) -> u8 {
    match s {
        Shape::Dot => 0,
        Shape::Line(a, ..) | Shape::Box3(a, ..) => *a,
    }
}

fn last(s: &Shape) -> u8 {
    match s {
        Shape::Dot => 0,
        Shape::Line(.., z) | Shape::Box3(.., z) => *z,
    }
}

// Of none: `Line(a, b, ..)` names both.
fn ends(s: &Shape) -> u8 {
    match s {
        Shape::Box3(a, .., c) => *a + *c,
        Shape::Line(a, b, ..) => *a + *b,
        Shape::Dot => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let b = Shape::Box3(1, 2, 3);
    let l = Shape::Line(4, 5);
    out.line(&kind(&Shape::Dot))?; // expect: 0
    out.line(&kind(&l))?; // expect: 1
    out.line(&first(&b))?; // expect: 1
    out.line(&last(&b))?; // expect: 3
    out.line(&ends(&b))?; // expect: 4
    out.line(&ends(&l))?; // expect: 9
    let t = (1u8, 2u8, 3u8, 4u8);
    let (x, ..) = t;
    let (.., y) = t;
    let (p, .., q) = t;
    let (r, s, u, v, ..) = t;
    out.line(&(x + y + p + q + r + s + u + v))?; // expect: 20
    let a = [5u8, 6, 7, 8];
    let [f, ..] = a;
    let [.., g] = a;
    let [h, .., i] = a;
    out.line(&(f + g + h + i))?; // expect: 26
    let o = Some((9u8, true, 'c'));
    if let Some((n, ..)) = o {
        out.line(&n)?; // expect: 9
    }
    match [1u8, 2, 3] {
        [1, ..] => out.line(&"one")?, // expect: one
        [.., 3] => out.line(&"three")?,
        [..] => out.line(&"else")?,
    }
    Ok(())
}
