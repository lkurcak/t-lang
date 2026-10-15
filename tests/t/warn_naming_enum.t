// Enums are named in UpperCamelCase, and so are their variants.
enum light_color { Red, yellow, GREEN_LIGHT }

fn code(c: light_color) -> int {
    match c {
        light_color::Red => 1,
        light_color::yellow => 2,
        light_color::GREEN_LIGHT => 3,
    }
}

fn main() {
    print(code(light_color::Red) + code(light_color::yellow) + code(light_color::GREEN_LIGHT));
}
// warning: type `light_color` should have an upper camel case name: `LightColor`
// warning: variant `yellow` should have an upper camel case name: `Yellow`
// warning: variant `GREEN_LIGHT` should have an upper camel case name: `GreenLight`
// expect: 6
