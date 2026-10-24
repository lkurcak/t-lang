// `match` on every kind of value, with nested patterns.

#[derive(Discard)]
enum Color { Red, Green, Blue }
#[derive(Discard)]
struct Pixel { color: Color, bright: bool }
#[derive(Discard)]
enum Opt { None, Some(i64) }

fn describe(p: Pixel) -> i64 {
    match p {
        Pixel { color: Color::Red, bright: true } => 1,
        Pixel { color: Color::Red, .. } => 2,
        Pixel { bright: false, .. } => 3,
        Pixel { color, bright: true } => match color {
            Color::Green => 4,
            _ => 5,
        },
    }
}

fn classify(n: i64) -> i64 with panic {
    match n {
        0 => 100,
        -1 => 200,
        other => other * 2,
    }
}

fn both(a: bool, b: bool) -> i64 {
    // Tuples would let us match both at once. Without them, nest the matches.
    match a {
        true => match b { true => 3, false => 2 },
        false => match b { true => 1, false => 0 },
    }
}

fn unwrap_or(o: Opt, default: i64) -> i64 {
    match o {
        Opt::Some(x) => x,
        Opt::None => default,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&describe(Pixel { color: Color::Red, bright: true }))?;    // expect: 1
    out.line(&describe(Pixel { color: Color::Red, bright: false }))?;   // expect: 2
    out.line(&describe(Pixel { color: Color::Blue, bright: false }))?;  // expect: 3
    out.line(&describe(Pixel { color: Color::Green, bright: true }))?;  // expect: 4
    out.line(&describe(Pixel { color: Color::Blue, bright: true }))?;   // expect: 5
    out.line(&classify(0))?;         // expect: 100
    out.line(&classify(-1))?;        // expect: 200
    out.line(&classify(21))?;        // expect: 42
    out.line(&both(true, false))?;   // expect: 2
    out.line(&unwrap_or(Opt::Some(7), 0))?;  // expect: 7
    out.line(&unwrap_or(Opt::None, 9))?;     // expect: 9
    match () { () => out.line(&1i64)? }         // expect: 1
    let k = match Opt::Some(3) { Opt::Some(n) => { let m = n * n; m } Opt::None => 0 };
    out.line(&k)?;                           // expect: 9
    Result::Ok(())
}
