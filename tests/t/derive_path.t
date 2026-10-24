// A derive names its traits as any path does: by a path, or by a name in
// scope, an import's too.
use core::cmp::Eq as Same;

#[derive(core::mem::Discard, core::mem::Copy, Same)]
struct P { x: i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { x: 1 };
    let q = p;
    out.line(&(p == q))?; // expect: true
    Result::Ok(())
}
