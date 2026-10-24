// The closure holds what `f` returns, which is that closure: a type that
// would hold itself.
fn f(n: i64) -> impl Fn() -> i64 with panic + Discard with panic {
    let g = f(n - 1);
    move || g() + 1
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic { let mut out = os.stdout; out.line(&f(1)())?;  Result::Ok(()) }
// error: the type `f` returns would contain itself
