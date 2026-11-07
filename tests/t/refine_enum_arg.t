// A refinement can be a type argument of an enum: each field of a variant
// whose type is that parameter has it (docs/53-refined-type-arguments.md).

fn digit(c: u8) -> Option<{ d: u8 | d < 10 }> {
    if c >= 48 && c <= 57 {
        Some(c - 48)
    } else {
        None
    }
}

// What the payload is known to be, in the arm that matched it.
fn value(c: u8) -> u8 {
    let table: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    match digit(c) {
        Some(d) => table[d as usize],
        None => 0,
    }
}

// Both type arguments of a `Result`.
fn split(n: u8) -> Result<{ v: u8 | v < 10 }, { v: u8 | v >= 10 }> {
    if n < 10 { Ok(n) } else { Err(n) }
}

fn tens(n: u8) -> u8 {
    match split(n) {
        Ok(_) => 0,
        Err(v) => v / 10 - 1 + 1,
    }
}

// A parameter's: the caller proves it of what it passes, a literal or
// what a refined result gave.
fn at(a: &[u8; 4], i: Option<{ v: usize | v < 4 }>) -> u8 {
    if let Some(i) = i { a[i] } else { 0 }
}

fn small(n: usize) -> Option<{ v: usize | v < 4 }> {
    if n < 4 { Some(n) } else { None }
}

// A `let`'s, and an enum in an enum.
fn nested(n: usize) -> usize {
    let o: Option<Option<{ v: usize | v < 4 }>> = Some(small(n));
    let a: [usize; 4] = [10, 20, 30, 40];
    match o {
        Some(Some(i)) => a[i],
        _ => 0,
    }
}

// An enum of the program's own, a payload of a payload, and a measure.
#[derive(Discard)]
enum Shape<T> { Dot(T), Line(T, T), Empty }

fn ends(s: &[u8]) -> Shape<{ i: usize | i < s.len() }> {
    if s.len() == 0 {
        Shape::Empty
    } else if s.len() == 1 {
        Shape::Dot(0)
    } else {
        Shape::Line(0, s.len() - 1)
    }
}

fn sum_ends(s: &[u8]) -> u16 {
    match ends(s) {
        Shape::Dot(i) => s[i] as u16,
        Shape::Line(a, b) => s[a] as u16 + s[b] as u16,
        Shape::Empty => 0,
    }
}

// A struct's field: the refinement names an earlier field.
#[derive(Discard)]
struct Cursor {
    len: usize,
    at: Option<{ i: usize | i < len }>,
}

fn cursor(len: usize) -> Cursor {
    match len {
        0 => Cursor { len, at: None },
        _ => Cursor { len, at: Some(len - 1) },
    }
}

fn last(c: &Cursor) -> usize {
    match c.at {
        Some(i) => c.len - i,
        None => 0,
    }
}

// A value's variant and payload, without a refinement: `Some(3)` is known
// to be `Some` and its payload 3.
fn third() -> u8 {
    let o = Some(3u8);
    let t: [u8; 5] = [1, 2, 3, 4, 5];
    match o {
        Some(i) => t[i as usize],
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&value(55))?; // expect: 7
    out.line(&value(65))?; // expect: 0
    out.line(&tens(42))?; // expect: 4
    out.line(&tens(4))?; // expect: 0
    let a: [u8; 4] = [5, 6, 7, 8];
    out.line(&at(&a, Some(2)))?; // expect: 7
    out.line(&at(&a, small(3)))?; // expect: 8
    out.line(&at(&a, None))?; // expect: 0
    out.line(&nested(1))?; // expect: 20
    out.line(&nested(9))?; // expect: 0
    let s: [u8; 3] = [1, 2, 3];
    out.line(&sum_ends(s.as_slice()))?; // expect: 4
    let one: [u8; 1] = [9];
    out.line(&sum_ends(one.as_slice()))?; // expect: 9
    out.line(&last(&cursor(5)))?; // expect: 1
    out.line(&last(&cursor(0)))?; // expect: 0
    out.line(&third())?; // expect: 4
    Ok(())
}
