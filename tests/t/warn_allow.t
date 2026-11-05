// `#[allow(..)]` turns lints off in what it's on, all of it: a function's
// parameters, body and nested functions, a type's fields and variants.

#[allow(non_camel_case_types, dead_code)]
#[derive(Discard)]
enum light_color {
    Red,
    Yellow_ish,
    Off,
}

/// Documented, then allowed.
#[allow(non_snake_case)]
#[derive(Discard)]
struct Pixel {
    R: i64,
    G: i64,
    #[allow(dead_code)]
    B: i64,
}

#[derive(Discard)]
struct Point {
    x: i64,
    #[allow(non_snake_case)]
    Y: i64,
}

#[allow(non_snake_case, unused_variables)]
fn sumPixel(pIxel: Pixel) -> i64 with panic {
    let unUsed = 0i64;
    fn innerHelper(nUm: i64) -> i64 {
        nUm
    }
    innerHelper(pIxel.R) + pIxel.G
}

#[allow(dead_code)]
fn never_called() {}

#[allow(dead_cod, unused_variable, nonsense)]
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let c = light_color::Yellow_ish;
    out.line(&sumPixel(Pixel { R: 1, G: 2, B: 3 }))?; // expect: 3
    let p = Point { x: 1, Y: 2 };
    out.line(&(p.x + p.Y))?; // expect: 3
    // Only in what it's on.
    let unused: i64 = 0;
    Ok(())
}
// warning: unknown lint `dead_cod`
// warning: unknown lint `unused_variable`
// warning: unknown lint `nonsense`
// warning: unused variable `c`
// warning: unused variable `unused`
