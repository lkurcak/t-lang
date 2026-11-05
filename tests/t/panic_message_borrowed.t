// A panic's message needn't be a literal.
fn fail(why: &str) -> ! with panic { panic(why) }

fn main() with panic {
    let s = "short";
    match s.split_at(2) {
        Some(p) => fail(p.tail),
        None => {},
    }
}
// error: runtime error: panicked: ort
