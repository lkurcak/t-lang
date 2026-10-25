// Dropping an array drops each of its values: the interpreter fails on a
// box that's never freed, or freed twice.
use alloc::boxed::Box;

fn pick(a: [Box<i64>; 3]) -> i64 {
    let [_, b, _] = a;
    *b
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a = [Box::new(1i64), Box::new(2i64)];
    out.line(&(*a[1]))?; // expect: 2
    let mut c = [Box::new(3i64), Box::new(4i64), Box::new(5i64)];
    c[0] = Box::new(6i64);
    out.line(&pick(c))?; // expect: 4
    let n: [[Box<i64>; 1]; 2] = [[Box::new(7i64)], [Box::new(8i64)]];
    out.line(&(*n[1][0]))?; // expect: 8
    Result::Ok(())
}
