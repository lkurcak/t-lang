// In generic code, a function type with an effect parameter has an impl
// for every set only if one says so: `with e` matches `with e`.
trait Name {
    fn name(&self) -> u32;
}

impl<effect e> Name for fn(u32) -> u32 with e {
    fn name(&self) -> u32 { 3 }
}

fn call<effect e>(f: fn(u32) -> u32 with e) -> u32 { f.name() }

fn one(x: u32) -> u32 with {} { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&call(one))?; // expect: 3
    Ok(())
}
