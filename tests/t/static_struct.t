// A static of a struct: its frozen fields can be read as any value's, and
// never change; its atomic one can.
use core::sync::Atomic;

#[derive(Discard, Sync)]
struct Stats { name: &'static str, limit: u32, hits: Atomic<u32> }

static STATS: Stats = Stats { name: "hits", limit: 2, hits: Atomic::new(0) };

fn hit() -> bool with global {
    let s: &'static Stats = &STATS;
    s.hits.fetch_add(1) < s.limit
}

fn main(os: std::Os) -> Result<(), std::io::Error> with global {
    let mut out = os.stdout;
    out.line(&hit())?;             // expect: true
    out.line(&hit())?;             // expect: true
    out.line(&hit())?;             // expect: false
    out.line(&STATS.name)?;        // expect: hits
    out.line(&STATS.hits.load())?; // expect: 3
    Ok(())
}
