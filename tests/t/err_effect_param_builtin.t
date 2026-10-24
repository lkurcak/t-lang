// `panic` is an effect already: as a parameter's name, `with panic` would
// mean two things.
fn apply<effect panic, F: Fn() with panic + Discard>(f: F) with panic { f() }

fn main() {}
// error: `panic` is a built-in effect, so it can't be an effect parameter's name
