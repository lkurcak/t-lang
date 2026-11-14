// A closure that calls a `&mut self` method on `c` and changes `c.n`
// captures `c` uniquely: changing what's through `c` doesn't need `c`
// declared `mut`, as merging the two captures keeps.
#[derive(Copy, Discard)]
struct C { n: u8, m: u8 }

impl C {
    fn touch(&mut self) {
        self.m = self.m / 2;
    }
}

fn f(c: &mut C) {
    let mut t = || {
        c.touch();
        c.n = 0;
    };
    t();
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut c = C { n: 5, m: 8 };
    f(&mut c);
    out.line(&f"{c.n} {c.m}") // expect: 0 4
}
