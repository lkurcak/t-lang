// A `&mut` a pattern binds into a variant's field, an array's element or
// either alternative of an or-pattern is a `&mut` of that place: what's
// written through it must keep what's declared there, and then it can be.
// (Step 62 refused these bindings outright.)
#[derive(Copy, Discard)]
enum E<T> { A(T), B(T) }

fn poke(o: &mut Option<{ s: u8 | s < 64 }>) {
    match o {
        Some(a) => *a = 5,
        None => {},
    }
}

fn halve(o: &mut Option<{ s: u8 | s < 64 }>) {
    if let Some(a) = o {
        *a = *a / 2;
    }
}

fn either(e: &mut E<{ s: u8 | s < 64 }>) {
    match e {
        E::A(a) | E::B(a) => *a = 63,
    }
}

fn get<'a>(t: &'a mut (Option<{ s: u8 | s < 64 }>, u8)) -> &'a mut (Option<{ s: u8 | s < 64 }>, u8) {
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut o: Option<{ s: u8 | s < 64 }> = Some(40);
    halve(&mut o);
    out.line(&o.unwrap_or(0))?; // expect: 20
    poke(&mut o);
    out.line(&o.unwrap_or(0))?; // expect: 5
    let mut e: E<{ s: u8 | s < 64 }> = E::B(3);
    let _ = E::A(0u8);
    either(&mut e);
    match e {
        E::A(x) | E::B(x) => out.line(&x)?, // expect: 63
    }
    let mut arr: [{ s: u8 | s < 64 }; 2] = [1, 2];
    match &mut arr {
        [a, _] => *a = 33,
    }
    out.line(&arr[0])?; // expect: 33
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (Some(1), 2);
    match get(&mut t) {
        (a, _) => *a = Some(7),
    }
    out.line(&t.0.unwrap_or(0))?; // expect: 7
    Ok(())
}
