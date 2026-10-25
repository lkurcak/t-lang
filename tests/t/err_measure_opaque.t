// Outside its module, a measure is what's been learned of it, not its
// body: changing the body can't break what's outside.
mod m {
    #[derive(Copy, Discard)]
    pub struct W { pub n: usize }

    impl W {
        #[measure]
        pub fn len(&self) -> usize { self.n }
    }

    pub fn at(w: &W, i: { i: usize | i < w.len() }) -> usize { i }
}

fn main() {
    let w = m::W { n: 3 };
    let _x = m::at(&w, 1);
}
// error: can't prove `i < w.len()` for argument `i` of `at`
