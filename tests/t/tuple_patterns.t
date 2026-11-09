// Tuple patterns: in `let`, `match`, `for`, nested, with `_`, and
// through references, which bind references.
#[derive(Copy, Discard)]
enum Light { Red, Green }

fn both(a: Option<u8>, b: Option<u8>) -> u8 {
    match (a, b) {
        (Some(x), Some(y)) => x + y,
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => 0,
    }
}

fn go(l: Light, walk: bool) -> &'static str {
    match (l, walk) {
        (Light::Green, true) => "walk",
        (Light::Green, false) | (Light::Red, _) => "wait",
    }
}

fn first_of(p: &(u8, (char, bool))) -> char {
    let (_, (c, _)) = p;
    *c
}

fn bump(p: &mut (i64, i64)) {
    let (a, b) = p;
    *a += 1;
    *b -= 1;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&both(Some(2), Some(3)))?; // expect: 5
    out.line(&both(None, Some(4)))?;    // expect: 4
    out.line(&both(None, None))?;       // expect: 0
    out.line(&go(Light::Green, true))?; // expect: walk
    out.line(&go(Light::Red, true))?;   // expect: wait
    out.line(&first_of(&(1, ('z', false))))?; // expect: z
    let mut p = (10, 20);
    bump(&mut p);
    out.line(&p.0)?;                    // expect: 11
    out.line(&p.1)?;                    // expect: 19
    let pairs: [(u8, u8); 3] = [(1, 2), (3, 4), (5, 6)];
    let mut sum: u8 = 0;
    for (a, b) in pairs.iter() {
        sum += *a * *b;
    }
    out.line(&sum)?;                    // expect: 44
    let Some((x, y)) = Some((7u8, 8u8)) else { return Ok(()) };
    out.line(&(x * y))?;                // expect: 56
    if let (1, z) = (1u8, 'q') {
        out.line(&z)?;                  // expect: q
    }
    Ok(())
}
