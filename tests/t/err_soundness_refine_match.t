// In the second arm, `d` is anything but 0, not 0.
fn zero(z: { v: int | v == 0 }) -> int { z }

fn f(d: int) -> int {
    match d {
        0 => 0,
        _ => zero(d),
    }
}

fn main() {
    print(f(5));
}
// error: can't prove `v == 0` for argument `z` of `zero`
