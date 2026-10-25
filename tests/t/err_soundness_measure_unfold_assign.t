// In its module, a measure is its body: after a write to a field it reads,
// it's the new value's.
mod m {
    #[derive(Copy, Discard)]
    pub struct W<'a> { s: &'a [i64], n: { n: usize | n <= s.len() } }

    impl<'a> W<'a> {
        #[measure]
        pub fn len(&self) -> usize { self.n }

        pub fn index(&self, i: { i: usize | i < self.len() }) -> &i64 { &self.s[i] }

        pub fn first(&mut self) -> i64 {
            if self.len() > 0 {
                self.n = 0;
                return *self.index(0);
            }
            0
        }
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
