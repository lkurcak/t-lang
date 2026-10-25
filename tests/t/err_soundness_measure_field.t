// In its module a measure is its body, so a write to a field it reads
// changes it.
mod m {
    #[derive(Copy, Discard)]
    pub struct W<'a> { s: &'a [i64], n: { n: usize | n <= s.len() } }

    impl<'a> W<'a> {
        #[measure]
        pub fn len(&self) -> usize { self.n }

        pub fn index(&self, i: { i: usize | i < self.len() }) -> &i64 { &self.s[i] }

        pub fn set_s(&mut self, s: &'a [i64]) {
            if self.n > 0 {
                self.s = s;
            }
        }
    }
}

fn main() {}
// error: can't prove `n <= s.len()` for field `n` of `W`, which names `s`
