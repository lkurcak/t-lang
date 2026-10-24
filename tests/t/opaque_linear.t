// What a function returns needn't be `Discard`: then it must be used up,
// here by calling it.

use alloc::boxed::Box;

fn once(b: Box<i64>) -> impl FnOnce() -> i64 { move || *b }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let f = once(Box::new(4));
    out.line(&f())?;                     // expect: 4
    out.line(&once(Box::new(5))())?;     // expect: 5
    Result::Ok(())
}
