// Enums are named in UpperCamelCase, and so are their variants.
#[derive(Discard)]
enum light_color { Red, yellow, GREEN_LIGHT }

fn code(c: light_color) -> i64 {
    match c {
        light_color::Red => 1,
        light_color::yellow => 2,
        light_color::GREEN_LIGHT => 3,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&(code(light_color::Red) + code(light_color::yellow) + code(light_color::GREEN_LIGHT)))?;
    Result::Ok(())
}
// warning: type `light_color` should have an upper camel case name: `LightColor`
// warning: variant `yellow` should have an upper camel case name: `Yellow`
// warning: variant `GREEN_LIGHT` should have an upper camel case name: `GreenLight`
// expect: 6
