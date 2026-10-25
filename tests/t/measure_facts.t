// What's known of a value's measures goes where the value goes: a copy has
// the same length, a refinement of a result says it, and a struct's
// measures, in its module, are what its fields say.
mod m {
    #[derive(Copy, Discard)]
    pub struct Pair { a: usize, b: usize }

    impl Pair {
        pub fn new(a: usize, b: usize) -> Pair { Pair { a, b } }

        #[measure]
        pub fn a(&self) -> usize { self.a }

        #[measure]
        pub fn ordered(&self) -> bool { self.a <= self.b }

        /// The pair, in order.
        pub fn sorted(self) -> { p: Pair | p.ordered() } {
            if self.a <= self.b { self } else { Pair { a: self.b, b: self.a } }
        }

        pub fn gap(&self) -> usize {
            if self.ordered() { self.b - self.a } else { self.a - self.b }
        }
    }
}

fn longer<'a>(s: &'a [i64], t: &'a [i64]) -> { r: &'a [i64] | r.len() >= s.len() && r.len() >= t.len() } {
    if s.len() >= t.len() {
        return s;
    }
    t
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(5);
    l.push(6);
    let s = l.as_slice();
    if s.len() == 2 {
        let t = s;
        out.line(&t[1])?; // expect: 6
        let r = longer(t, core::slice::empty());
        out.line(&r[1])?; // expect: 6
    }
    let p = m::Pair::new(7, 3).sorted();
    out.line(&p.gap())?; // expect: 4
    out.line(&p.a())?; // expect: 3
    let b: alloc::boxed::Box<&[i64]> = alloc::boxed::Box::new(s);
    if b.len() > 0 {
        out.line(&b[0])?; // expect: 5
    }
    Result::Ok(())
}
