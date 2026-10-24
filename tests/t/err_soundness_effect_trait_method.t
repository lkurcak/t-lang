// A trait's method that says it may panic may panic, whichever impl runs.
trait Op {
    fn op(&self, n: i64) -> i64 with panic;
}

fn twice<T: Op>(t: &T, n: i64) -> i64 { t.op(t.op(n)) }

#[derive(Discard)]
struct Inc {}

impl Op for Inc {
    fn op(&self, n: i64) -> i64 with panic { n + 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&twice(&Inc {}, 1))?;
    Result::Ok(())
}
// error: `op` may panic, but `twice` doesn't say `with panic`
