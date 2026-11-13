// A reference to a program's type is the program's to implement a trait
// for, as the type is: `&Bag` and `&mut Bag` belong to whoever `Bag` does.
// Core's `impl Iterator for &mut I` needs `Bag: Iterator`, which only this
// program could say, and doesn't, so it isn't for `&mut Bag`.
#[derive(Discard)]
struct Bag { n: i64 }

impl<'a> Iterator for &'a mut Bag {
    type Item = i64;
    effect e = {};

    fn next(&mut self) -> Option<i64> {
        if self.n <= 0 {
            None
        } else {
            self.n -= 1;
            Some(self.n)
        }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut b = Bag { n: 3 };
    let mut total = 0;
    for x in &mut b {
        total += x;
    }
    out.line(&total)?; // expect: 3
    out.line(&b.n) // expect: 0
}
