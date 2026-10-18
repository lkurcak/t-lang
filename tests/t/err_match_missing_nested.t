#[derive(Discard)]
enum Color { Red, Green }
#[derive(Discard)]
struct Pixel { color: Color, bright: bool }
fn f(p: Pixel) -> int {
    match p {
        Pixel { color: Color::Red, .. } => 1,
        Pixel { color: Color::Green, bright: true } => 2,
    }
}
fn main() {}
// error: non-exhaustive `match`: `Pixel { color: Color::Green, bright: false }` not covered
