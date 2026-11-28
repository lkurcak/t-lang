// Another package's open bounds: its exported function does what the
// closure it's given does, its trait's method is implemented here with
// the same open bound, and its iterator's `e` is its closure's.
use alloc::boxed::Box;
use visits::{FromFn, Visit, each};

#[derive(Discard)]
struct Two {}

impl Visit for Two {
    fn visit<G: FnMut(u8) + Discard>(&self, mut g: G) with G::e {
        g(1);
        g(2);
    }
}

fn total<V: Visit>(v: &V) -> u8 with {} {
    let mut t = 0u8;
    v.visit(|x| t = core::num::wrapping_add(t, x));
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let mut sum = 0u8;
    each(4, |x| sum = sum + x);
    out.line(&sum)?;            // expect: 6
    out.line(&total(&Two {}))?; // expect: 3
    let mut boxes = 0u8;
    each(2, |x| {
        let b = Box::new(x);
        boxes = boxes + *b;
    });
    out.line(&boxes)?; // expect: 1
    let mut n = 0u8;
    let it = FromFn::new(|| {
        if n < 2 {
            n = n + 1;
            Some(n)
        } else {
            None
        }
    });
    for x in it {
        out.line(&x)?; // expect: 1
        // expect: 2
    }
    Ok(())
}
