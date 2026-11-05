// Only what implements `Parse` can be parsed.
#[derive(Discard)]
struct Point { x: i64, y: i64 }

fn main() {
    let _ = "1,2".parse::<Point>();
}
// error: `str::parse` needs its `T` to implement `core::str::Parse`, and `Point` doesn't
