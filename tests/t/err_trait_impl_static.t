// Found for a `&S` of any lifetime, so `get` would return a reference to
// a local as `'static`.
#[derive(Discard)]
struct S { x: i64 }

trait Get { fn get(self) -> &'static i64; }

impl Get for &'static S {
    fn get(self) -> &'static i64 { &self.x }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r;
    {
        let s = S { x: 5 };
        r = Get::get(&s);
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `&'static S` says a lifetime is `'static`, but an impl of a trait must hold at every lifetime, as finding the impl for a type doesn't look at lifetimes
