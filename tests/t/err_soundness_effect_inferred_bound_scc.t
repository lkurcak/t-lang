// The same from inside `g`'s own group, where `g`'s effects aren't known
// yet when its body is checked: the bound is checked once they are.
fn twice<F: Fn() -> i64 + Discard>(f: F) -> i64 { core::num::wrapping_add(f(), f()) }

fn g(n: u8) -> i64 {
    if n == 0 { panic("boom") } else { twice(|| g(n - 1)) }
}

fn main() with panic { let _ = g(1); }
// error: this closure may panic, but `twice` needs calls of it to fit
