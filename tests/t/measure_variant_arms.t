// A variant measure's arms are tried in order, `_` takes the rest, and a
// measure can count: which variant gives which number.
#[derive(Copy, Discard)]
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

    #[measure]
    fn wait(&self) -> u8 {
        match self {
            Light::Red => 30,
            Light::Amber | Light::Green => 0,
        }
    }
}

fn green() -> { l: Light | l.go() && l.wait() == 0 } { Light::Green }

fn red() -> { l: Light | !l.go() && l.wait() == 30 } { Light::Red }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [u8; 31] = [7; 31];
    out.line(&a[red().wait() as usize])?; // expect: 7
    let g: { b: bool | b } = green().go();
    out.line(&g)?;                        // expect: true
    let _ = Light::Amber;
    Ok(())
}
