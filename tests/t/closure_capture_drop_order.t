// What a closure moved in is dropped with it; the rest of the variable,
// where the variable is: `c`, made after `p`, is dropped first, with
// `p.b`, then `p`, with `p.a`, so `p.a` is the last.
use core::cell::Cell;

mod noisy {
    pub struct Noisy<'a> { pub last: &'a core::cell::Cell<u32>, pub id: u32 }

    impl<'a> Discard for Noisy<'a> {
        fn discard(self) {
            self.last.set(self.id);
        }
    }
}

#[derive(Discard)]
struct Pair<'a> { a: noisy::Noisy<'a>, b: noisy::Noisy<'a> }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let last = Cell::new(0u32);
    {
        let p = Pair {
            a: noisy::Noisy { last: &last, id: 1 },
            b: noisy::Noisy { last: &last, id: 2 },
        };
        let c = move || {
            let b = &p.b;
            b.id
        };
        let _a = p.a.id;
        let _b = c();
    }
    out.line(&last.get()) // expect: 1
}
