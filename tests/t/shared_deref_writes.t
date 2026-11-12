// Writes through a `&mut` behind a `&mut`, or a `&mut` held in a struct
// reached by `&mut`, are fine; so is a `Cell` behind any `&`.
use core::cell::Cell;

#[derive(Discard)]
struct H<'a> {
    r: &'a mut usize,
}

fn poke(h: &mut H) {
    *h.r = 5;
}

fn bump(a: &mut &mut usize) {
    **a = 6;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut x: usize = 1;
    let mut h = H { r: &mut x };
    poke(&mut h);
    let mut r = &mut x;
    bump(&mut r);
    out.line(&x)?; // expect: 6
    let c: Cell<usize> = Cell::new(1);
    let a = &c;
    let b = &a;
    b.set(3);
    out.line(&c.get())?; // expect: 3
    Result::Ok(())
}
