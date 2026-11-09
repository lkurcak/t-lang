// `checked_mul` is written in T since step 51: it agrees with arithmetic in
// a wider type on the edges of i8 and u8, and of i64 against -1.
use core::fmt::Display;
use core::num::{greatest, least};

fn show<T: Int + Display>(out: &mut std::io::Stdout, r: Option<T>) -> Result<(), std::io::Error> {
    match r {
        Some(n) => out.line(&n),
        None => out.line("none"),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let cases: [i8; 9] = [0, 1, -1, 2, -2, 11, -11, 127, -128];
    let mut n: u32 = 0;
    for a in cases.iter() {
        for b in cases.iter() {
            let want = *a as i32 * *b as i32;
            let ok = want >= -128 && want <= 127;
            match checked_mul(*a, *b) {
                Some(p) => if !ok || p as i32 != want {
                    out.line(&f"bad {a} {b}")?;
                },
                None => if ok {
                    out.line(&f"missed {a} {b}")?;
                },
            }
            n = n + 1;
        }
    }
    let us: [u8; 5] = [0, 1, 2, 15, 255];
    for a in us.iter() {
        for b in us.iter() {
            let want = *a as u32 * *b as u32;
            match checked_mul(*a, *b) {
                Some(p) => if want > 255 || p as u32 != want {
                    out.line(&f"bad {a} {b}")?;
                },
                None => if want <= 255 {
                    out.line(&f"missed {a} {b}")?;
                },
            }
        }
    }
    show(&mut out, checked_mul(greatest::<i64>(), -1))?; // expect: -9223372036854775807
    show(&mut out, checked_mul(least::<i64>(), -1))?;    // expect: none
    out.line(&n)                                         // expect: 81
}
