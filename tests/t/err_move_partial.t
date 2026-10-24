use alloc::boxed::Box;

#[derive(Discard)]
struct Owner { a: Box<i64>, b: Box<i64> }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let o = Owner { a: Box::new(1), b: Box::new(2) };
    let a = o.a;
    out.line(&(*a))?;
    let _p = o;
    Result::Ok(())
}
// error: use of partly moved value `o`: `o.a` was moved out of it
