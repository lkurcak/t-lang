// A tuple pattern matches when each of its parts does. The first arm
// says nothing the logic can follow of the string, so its not matching
// says nothing of the option either: in the second arm, `a` is any number
// below 200. Were the first arm's condition taken to be its `Some` alone,
// the second arm would be unreachable, and anything would follow there.
fn need(s: { s: u8 | s < 64 }) -> u8 { s }

fn pick(s: &str, o: Option<{ v: u8 | v < 200 }>) -> u8 {
    match (s, o) {
        ("x", Some(_)) => 0,
        (_, Some(a)) => need(a),
        (_, None) => 0,
    }
}

fn main() {
    let _ = pick("y", Some(100));
}
// error: can't prove `s < 64` for argument `s` of `need`
