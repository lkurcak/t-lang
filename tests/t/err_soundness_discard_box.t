// Through a box, an option and a closure, a drop still reaches the
// `discard` that reads `x`.
use alloc::boxed::Box;

struct Peek<'a> { r: &'a i64, out: &'a mut std::io::Stdout }

impl<'a> Discard for Peek<'a> {
    fn discard(self) {
        let _ = self.out.line(self.r);
    }
}

fn main(os: std::Os) with alloc {
    let mut out = os.stdout;
    let _f;
    {
        let x = 5;
        let b = Option::Some(Box::new(Peek { r: &x, out: &mut out }));
        _f = move || { let _b = &b; };
    }
}
// error: `x` doesn't live long enough
