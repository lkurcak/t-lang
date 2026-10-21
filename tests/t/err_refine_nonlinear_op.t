// True, but the checker only reasons linearly: `x * x` is a product of two
// unknowns, so it can't prove it fits.
fn square(x: { v: int | v >= 0 && v < 100 }) -> int { x * x }

fn main() {
    print(square(3));
}
// error: `*` may panic, but `square` doesn't say `with panic`
