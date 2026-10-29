// What's inferred is the least the body needs: `inc` can't panic, so it's
// as if it said nothing, and meets a bound that allows no effects.
fn inc(n: i64) -> i64 { core::num::wrapping_add(n, 1) }

fn twice<F: Fn(i64) -> i64 + Discard>(f: F, x: i64) -> i64 { f(f(x)) }

export fn total(x: i64) -> i64 { twice(inc, x) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&total(1))?; // expect: 3
    Result::Ok(())
}
