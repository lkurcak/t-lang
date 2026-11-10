// A derived `Copy` takes the type's `Fn` bound for granted, as it holds of
// every type that's well-formed: so a pair of a closure that may panic is
// refused where it's made, before it's ever copied.
#[derive(Discard, Copy where F: Copy)]
struct Pair<F: Fn(u8) -> u8> { f: F, n: u8 }

fn use_pair<F: Fn(u8) -> u8 + Copy>(p: Pair<F>) -> u8 { (p.f)(p.n) }

fn main() with panic {
    // `x / (x - x)` divides by zero.
    let p = Pair { f: |x: u8| x / (x - x), n: 9 };
    let _a = use_pair(p);
    let _b = use_pair(p);
}
// error: this closure may panic, but `Pair` needs calls of it to fit `fn(u8) -> u8`, which can't panic
