// A variant measure is about which variant the value is: a `None` isn't
// `is_some`, and the first arm that matches is the one.
#[derive(Discard)]
enum Light {
    Red,
    Amber,
    Green,
}

impl Light {
    #[measure]
    fn go(&self) -> bool {
        match self {
            Light::Green => true,
            _ => false,
        }
    }
}

fn red() -> { l: Light | l.go() } { Light::Red }

fn main() {
    let _ = red();
    let _ = Light::Amber;
}
// error: can't prove `l.go()` for the result of `red`
