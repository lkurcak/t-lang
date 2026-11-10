// The `Some`s in a tuple or struct pattern say which variants the values
// are, so what's known of a variant's fields holds in the arm: two refined
// squares at once, by `match`, `let else` and through a reference.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn both(a: u8, b: u8) -> u8 {
    match (square(a), square(b)) {
        (Some(x), Some(y)) => need(x) + need(y),
        (Some(x), None) => need(x),
        (None, Some(y)) => need(y),
        (None, None) => 0,
    }
}

fn first(a: u8, b: u8) -> u8 {
    let (Some(x), Some(_)) = (square(a), square(b)) else { return 0 };
    need(x)
}

fn either(a: u8, b: u8) -> u8 {
    match (square(a), square(b)) {
        (None, _) | (_, None) => 0,
        (Some(x), Some(y)) => need(x) * 0 + need(y),
    }
}

fn borrowed(a: u8, b: u8) -> u8 {
    let t = (square(a), square(b));
    match &t {
        (_, Some(y)) => need(*y),
        _ => 0,
    }
}

#[derive(Discard)]
struct Move { from: Option<u8>, to: Option<{ s: u8 | s < 64 }> }

// Fields named in another order than they're declared in.
fn to(m: Move) -> u8 {
    match m {
        Move { to: Some(t), from: None } => need(t),
        Move { to: Some(t), .. } => need(t),
        _ => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&both(3, 4))?; // expect: 7
    out.line(&both(3, 99))?; // expect: 3
    out.line(&both(99, 5))?; // expect: 5
    out.line(&first(10, 20))?; // expect: 10
    out.line(&first(10, 200))?; // expect: 0
    out.line(&either(1, 2))?; // expect: 2
    out.line(&borrowed(1, 9))?; // expect: 9
    out.line(&to(Move { from: Some(7), to: square(8) }))?; // expect: 8
    Ok(())
}
