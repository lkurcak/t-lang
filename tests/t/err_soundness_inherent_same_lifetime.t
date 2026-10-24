// An inherent impl for `P<'a, 'a>`: its method, called on a `P` whose two
// lifetimes differ, makes them the same, so `x` can't outlive `short`.

use alloc::boxed::Box;

#[derive(Discard)]
struct P<'a, 'b> { x: &'a i64, y: &'b i64 }

impl<'a> P<'a, 'a> {
    fn flip(&mut self) {
        let t = self.x;
        self.x = self.y;
        self.y = t;
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let long = Box::new(1);
    let x;
    {
        let short = Box::new(2);
        let mut p = P { x: &*long, y: &*short };
        p.flip();
        x = p.x;
    }
    out.line(&(*x))?;
    Result::Ok(())
}
// error: `short` doesn't live long enough
