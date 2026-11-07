// A product of two unknowns is a value of the logic, and what bounds one
// factor bounds the product, given the other's sign: `x <= 99` and
// `x >= 0` give `x * x <= 99 * x`.

// Each factor is below 100, so the product is below 100 * 100: it fits.
fn square(x: { v: i64 | v >= 0 && v < 100 }) -> i64 with {} { x * x }

// The cell in column `x` of row `y`, in a grid `w` wide and `h` high, row
// by row: before `w * h`, as `y <= h - 1` gives `y * w <= h * w - w`. That
// `w * h` fits is needed too, or `y * w` might not.
fn cell(
    w: usize,
    h: { h: usize | w * h <= 1000 },
    x: { x: usize | x < w },
    y: { y: usize | y < h },
) -> { i: usize | i < w * h } with {} { y * w + x }

// `w * h` and `h * w` are one value.
fn area(w: usize, h: { h: usize | w * h <= 1000 }) -> { a: usize | a <= 1000 } with {} { h * w }

// A negative times a positive is negative.
fn neg(a: { a: i32 | a < 0 && a > -100 }, b: { b: i32 | b > 0 && b < 100 }) -> { p: i32 | p < 0 }
with {}
{ a * b }

// A square is never negative, whatever the sign: `x <= 0` with `x <= 0`
// gives `x * x >= 0 * x`.
fn sq(x: { x: i8 | x > -10 && x < 10 }) -> { s: i8 | s >= 0 } with {} { x * x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&square(7))?;        // expect: 49
    out.line(&cell(3, 2, 1, 1))?; // expect: 4
    out.line(&area(10, 20))?;     // expect: 200
    out.line(&neg(-3, 4))?;       // expect: -12
    out.line(&sq(-9))?;           // expect: 81
    Ok(())
}
