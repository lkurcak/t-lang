// A struct's `&mut` field, through a `&` to the struct, is shared.
#[derive(Discard)]
struct H<'a> {
    r: &'a mut usize,
}

fn poke(h: &H) {
    *h.r = 5;
}

fn main() {
    let mut x: usize = 1;
    let h = H { r: &mut x };
    poke(&h);
}
// error: cannot assign to `*h.r`: it's behind a `&` reference, `h`
