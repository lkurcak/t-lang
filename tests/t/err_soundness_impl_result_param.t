// A call through a type parameter is the trait's, even where one impl's
// header would match it: `T` may be `char`, whose impl isn't the blanket
// one. So the blanket impl's refined result says nothing at `pick`.
sealed trait Small {}

impl Small for u8 {}

trait Digit {
    fn digit(&self) -> u8;
}

impl<T: Small> Digit for T {
    fn digit(&self) -> { d: u8 | d < 10 } { 3 }
}

impl Digit for char {
    fn digit(&self) -> u8 { 200 }
}

fn pick<T: Digit>(x: &T) -> u8 {
    let squares: [u8; 10] = [0, 1, 4, 9, 16, 25, 36, 49, 64, 81];
    squares[x.digit() as usize]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&pick(&'x'))?;
    Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
