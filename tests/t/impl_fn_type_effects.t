// Two impls for function types that differ only in their effects are two
// impls, each with its own associated type and methods: monomorphisation
// names their copies apart (docs/81).
trait Out {
    type O;
    fn out(&self) -> Self::O;
}

impl Out for fn(u32) -> u32 {
    type O = bool;
    fn out(&self) -> bool { (*self)(1) == 1 }
}

impl Out for fn(u32) -> u32 with panic {
    type O = u32;
    fn out(&self) -> u32 { 7 }
}

fn get<T: Out>(t: &T) -> T::O { t.out() }

fn same(x: u32) -> u32 with {} { x }

fn div(x: u32) -> u32 with panic { 10 / x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f: fn(u32) -> u32 = same;
    let g: fn(u32) -> u32 with panic = div;
    let x: bool = get(&f);
    let y: u32 = get(&g);
    out.line(&x)?; // expect: true
    out.line(&y)?; // expect: 7
    Ok(())
}
