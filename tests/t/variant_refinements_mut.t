// A `&mut` a pattern binds to a variant's field whose refinement is about
// itself alone says it: what's written through it keeps it. So does a
// `&mut` to a struct's field whose refinement is about it alone. (Up to
// 0.80.0, a refined struct field couldn't be borrowed `&mut` but straight
// to a call.)
#[derive(Copy, Discard)]
enum Key {
    Digit({ d: u8 | d < 10 }),
    Other,
}

#[derive(Copy, Discard)]
struct S { a: { v: u8 | v < 3 }, b: u8 }

fn bump(k: &mut Key) {
    match k {
        Key::Digit(d) => {
            if *d < 9 {
                *d = *d + 1;
            }
        },
        Key::Other => {},
    }
}

fn set(r: &mut { v: u8 | v < 3 }) { *r = 2; }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut k = Key::Digit(4);
    bump(&mut k);
    let mut o = Key::Other;
    bump(&mut o);
    match k {
        Key::Digit(d) => out.line(&d)?, // expect: 5
        Key::Other => {},
    }
    let mut s = S { a: 1, b: 7 };
    let r = &mut s.a;
    *r = 0;
    out.line(&s.a)?; // expect: 0
    set(&mut s.a);
    out.line(&s.a)?; // expect: 2
    let S { a, .. } = &mut s;
    *a = 1;
    out.line(&(s.a + s.b))?; // expect: 8
    Ok(())
}
