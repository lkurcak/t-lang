// `run`'s effects are `{e, panic}`: its recursive call passes `boom`. So
// a total `g` doesn't make it total.
fn boom() { panic("boom") }

fn quiet() {}

fn run<effect e>(g: fn() with e, n: u8) {
    if n == 0 { g() } else { run(boom, n - 1) }
}

export fn safe() { run(quiet, 1) }

fn main() with panic { safe() }
// error: `run` may panic, but `safe` doesn't say `with panic`
