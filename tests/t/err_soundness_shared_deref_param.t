// A parameter `&&mut usize` gives no right to write.
fn poke(a: &&mut usize) {
    **a = **a + 1;
}

fn main() with panic {
    let mut x: usize = 1;
    let r = &mut x;
    poke(&r);
}
// error: cannot assign to `**a`: it's behind a `&` reference, `a`
