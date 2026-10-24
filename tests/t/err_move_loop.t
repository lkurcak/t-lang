use alloc::boxed::Box;

fn consume(b: Box<i64>) -> i64 { *b }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let b = Box::new(1);
    let mut i: i64 = 0;
    while i < 2 {
        out.line(&consume(b))?;
        i += 1;
    }
    Result::Ok(())
}
// error: use of moved value `b`
