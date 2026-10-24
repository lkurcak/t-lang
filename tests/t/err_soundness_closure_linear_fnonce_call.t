// A closure that owns `fd` but only reads it can't be `Fn`: every `Fn` is
// `FnOnce` too, and calling it that way would drop it after, `fd` and all.
// So it's only `FnOnce`, and its body must use `fd` up.
struct Fd { n: i64 }

fn once<F: FnOnce() -> i64>(f: F) -> i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = Fd { n: 3 };
    out.line(&once(move || fd.n))?;
    Result::Ok(())
}
// error: `fd` is never used up
