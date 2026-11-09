// The first arm that matches is the one: `_` is only what the arms before
// it didn't match, so a `Green` doesn't `stop`.
#[derive(Discard)]
enum Light {
    Red,
    Green,
}

impl Light {
    #[measure]
    fn stop(&self) -> bool {
        match self {
            Light::Green => false,
            _ => true,
        }
    }
}

fn green() -> { l: Light | l.stop() } { Light::Green }

fn main() {
    let _ = green();
    let _ = Light::Red;
}
// error: can't prove `l.stop()` for the result of `green`
