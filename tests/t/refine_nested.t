// What's inside a field a refinement is about can change, as long as the
// refinement still holds after: by assignment, or lent to a method that
// says what it does.
#[derive(Copy, Discard)]
struct Inner { b: usize }

impl Inner {
    #[measure]
    fn m(&self) -> usize { self.b }

    fn grow(&mut self) ensures self.m() >= old(self.m()) {
        if self.b < 100 {
            self.b = self.b + 1;
        }
    }
}

#[derive(Copy, Discard)]
struct S { a: Inner, n: { n: usize | n <= a.m() } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut s = S { a: Inner { b: 5 }, n: 5 };
    s.a.b = 7;
    s.a.grow();
    out.line(&s.a.b)?; // expect: 8
    out.line(&s.n)?; // expect: 5
    Result::Ok(())
}
