// A program's own measures: `#[measure]` on a method of `&self` whose body
// the logic can read. In its module, the checker knows what it is; outside,
// only what's been learned of it, by calling it or from refinements.
mod window {
    /// The first `n` values of `data`.
    #[derive(Copy, Discard)]
    pub struct Window<'a> { data: &'a [i64], n: { n: usize | n <= data.len() } }

    impl<'a> Window<'a> {
        pub fn new(data: &'a [i64]) -> Window<'a> { Window { data, n: data.len() } }

        /// How many values it has.
        #[measure]
        pub fn len(&self) -> usize { self.n }

        /// Has it none?
        #[measure]
        pub fn is_empty(&self) -> bool { self.len() == 0 }

        /// The value at `i`, which is less than `len()`: `data` has it, as
        /// `n <= data.len()`.
        pub fn index(&self, i: { i: usize | i < self.len() }) -> &i64 { &self.data[i] }

        /// The first `n` values, if it has as many.
        pub fn shorten(&self, n: usize) -> Window<'a> {
            if n <= self.n { Window { data: self.data, n } } else { *self }
        }
    }
}

use window::Window;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(1);
    l.push(2);
    l.push(3);
    let w = Window::new(l.as_slice()).shorten(2);
    let n = w.len();
    out.line(&n)?; // expect: 2
    if !w.is_empty() {
        out.line(&w[0])?; // expect: 1
    }
    if n > 1 {
        out.line(&w[n - 1])?; // expect: 2
    }
    Result::Ok(())
}
