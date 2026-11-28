// An impl's and a trait's method's open bounds have effect parameters of
// their own, decided by the type the impl is for, or by the argument, as
// written ones are.
#[derive(Discard where F: Discard)]
struct FromFn<F> { f: F }

impl<T, F: FnMut() -> Option<T>> Iterator for FromFn<F> {
    type Item = T;
    effect e = F::e;

    fn next(&mut self) -> Option<T> with F::e { (self.f)() }
}

trait Each {
    fn each<F: FnMut(u8) + Discard>(&self, f: F) with F::e;
}

#[derive(Discard)]
struct Three {}

// Its own name for the parameter: the trait's `F` is its `G`.
impl Each for Three {
    fn each<G: FnMut(u8) + Discard>(&self, mut g: G) with G::e {
        g(1);
        g(2);
        g(3);
    }
}

// Generic in what has the method: calls of it do what its `F` does.
fn sum<E: Each>(e: &E) -> u8 with {} {
    let mut s = 0u8;
    e.each(|x| s = core::num::wrapping_add(s, x));
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut n = 0u8;
    let it = FromFn {
        f: || {
            if n < 3 {
                n = n + 1;
                Some(n)
            } else {
                None
            }
        },
    };
    for x in it {
        out.line(&x)?; // expect: 1
        // expect: 2
        // expect: 3
    }
    let mut m = 0u8;
    (Three {}).each(|x| m = m + x);
    out.line(&m)?;              // expect: 6
    out.line(&sum(&Three {}))?; // expect: 6
    Ok(())
}
