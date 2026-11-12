// Two places with the same declaration can be swapped through generic
// code; so can two plain ones.
use alloc::list::List;
use core::mem::swap;

#[derive(Discard)]
struct W2<'a, T> {
    a: &'a mut T,
    b: &'a mut T,
}

fn sw<'a, T>(w: W2<'a, T>) {
    swap(w.a, w.b);
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    l.push(4);
    if let Some((a, b)) = l.as_mut_slice().split_at_mut(1) {
        if b.len() > 0 {
            sw(W2 { a: &mut a[0], b: &mut b[0] });
        }
    }
    out.line(&l[0])?; // expect: 4
    let mut x: usize = 1;
    let mut y: usize = 2;
    sw(W2 { a: &mut x, b: &mut y });
    out.line(&x)?; // expect: 2
    Result::Ok(())
}
