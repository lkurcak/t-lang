// `/` and `%` by a constant are in the logic: Presburger arithmetic,
// linear, as `a / k` is the integer `q` with `k * q` within `k - 1` of `a`,
// on the side of zero. They mean what they mean in a body: `/` rounds
// towards zero, and `%` has the sign of what's divided.

// t-metal's loop: a hexadecimal digit at a time, 4 bits down each round.
fn digits() -> u64 {
    let mut shift: u32 = 64;
    let mut count: u64 = 0;
    while shift > 0 invariant shift % 4 == 0 && shift <= 64 && count == 16 - shift / 4 {
        shift = shift - 4;
        count = count + 1;
    }
    count
}

// Rounding towards zero, by a negative divisor too.
fn seven(a: { v: i64 | v == -7 }) -> {
    r: i64 | r / 2 == -3 && r % 2 == -1 && r / -2 == 3 && r % -2 == -1 && 7 / -2 == -3
} {
    a
}

fn even(n: { v: u64 | v % 2 == 0 }) -> { r: u64 | r % 2 == 0 } { n + 2 }

// A body's `%` is the predicate's: the branch knows `n` is even.
fn next_even(n: u64) -> u64 {
    if n % 2 == 0 && n < 100 { even(n) } else { 0 }
}

// What a body divides is what a predicate does: `a % 10` is `a` less ten
// times `a / 10`.
fn last_digit(a: u64) -> { r: u64 | r == a % 10 && r < 10 } { a - (a / 10) * 10 }

// A quotient is an unknown of its own: a product of it and another is a
// product of two.
fn product(x: u8, y: u8, z: u8) -> { r: u64 | r == (x * y / 2) * z } {
    ((x as u64 * y as u64) / 2) * z as u64
}

// The two halves of an odd number are one apart.
fn halves(n: u64) -> { r: u64 | r <= n && n - r <= 1 } { n / 2 + n / 2 }

// What defines a quotient holds on every path: here, after the `if`
// where it was first said, that it's at most `a`.
fn after_join(a: u64, c: bool) -> { r: u64 | r <= a } {
    if c {
        let _h = a / 2;
    }
    a / 2
}

type Even = { v: u64 | v % 2 == 0 };

fn twice(n: u64) -> Even with panic { n * 2 }

// What an `old(..)` divided was, and a field's quotient.
#[derive(Copy, Discard)]
struct Grid { cells: { v: u64 | v % 8 == 0 } }

#[derive(Copy, Discard)]
struct Counter { n: u64 }

fn grow(c: &mut Counter) with panic ensures c.n / 2 == old(c.n) { c.n = c.n * 2; }

fn rows(g: Grid) -> { r: u64 | r * 8 == g.cells } { g.cells / 8 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&digits())?; // expect: 16
    out.line(&seven(-7))?; // expect: -7
    out.line(&next_even(4))?; // expect: 6
    out.line(&next_even(5))?; // expect: 0
    out.line(&last_digit(1234))?; // expect: 4
    out.line(&product(3, 5, 7))?; // expect: 49
    out.line(&halves(7))?; // expect: 6
    out.line(&after_join(9, true))?; // expect: 4
    out.line(&twice(21))?; // expect: 42
    let mut c = Counter { n: 21 };
    grow(&mut c);
    out.line(&c.n)?; // expect: 42
    out.line(&rows(Grid { cells: 64 }))?; // expect: 8
    Ok(())
}
