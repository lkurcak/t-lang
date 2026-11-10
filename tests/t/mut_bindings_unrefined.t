// A field bound by `&mut` through match ergonomics, where nothing is
// declared of what it's in, can be written through.
#[derive(Discard)]
struct S { o: Option<u8>, n: usize }

fn bump(o: &mut Option<u8>) {
    match o {
        Some(a) => *a = 9,
        None => {},
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x = Some(3u8);
    if let Some(a) = &mut x {
        *a = 4;
    }
    bump(&mut x);
    let mut s = S { o: Some(1), n: 2 };
    if let Some(a) = &mut s.o {
        *a = 5;
    }
    let mut t = (Some(1u8), 2u8);
    match &mut t {
        (Some(a), b) => {
            *a = 7;
            *b = 8;
        },
        _ => {},
    }
    out.line(&x.unwrap_or(0))?; // expect: 9
    out.line(&s.o.unwrap_or(0))?; // expect: 5
    out.line(&t.0.unwrap_or(0))?; // expect: 7
    out.line(&s.n)?; // expect: 2
    Ok(())
}
