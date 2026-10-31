// `#[test(panics)]` passes only by panicking: not by finishing, nor by
// giving back an `Err`. Its effects can be inferred, through what it calls.
fn main() {}

fn first(xs: &[u8]) -> u8 { *xs.get(0).unwrap() }

#[test(panics)]
fn out_of_bounds() {
    let xs: [u8; 0] = [];
    let _ = first(xs.as_slice());
}

#[test(panics)]
fn says_why() { panic("on purpose") }

#[test(panics)]
fn doesnt() {
    let xs: [u8; 1] = [7];
    assert_eq(&first(xs.as_slice()), &7);
}

#[derive(Discard)]
struct Oops {}

impl core::fmt::Display for Oops {
    fn fmt<W: core::fmt::Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str("oops")
    }
}

#[test(panics)]
fn errs() -> Result<(), Oops> {
    if first([1u8].as_slice()) == 2 { panic("no") }
    Result::Err(Oops {})
}

// Running out of stack isn't a panic, though it aborts too.
fn forever(n: u64) -> u64 {
    if n == 0 { panic("never") }
    forever(n + 1)
}

#[test(panics)]
fn overflows() { let _ = forever(1); }

// test: out_of_bounds ... ok
// test: says_why ... ok
// test: doesnt ... FAILED: runtime error: didn't panic, which this test passes only by doing
// test: errs ... FAILED: gave back an error: oops
// test: overflows ... FAILED: stack overflow
