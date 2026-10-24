// A lowercase type parameter is a type parameter, with a name that should
// be in UpperCamelCase. (It used to be taken for an effect parameter.)
fn id<t>(x: t) -> t { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&id(1i64))?; // expect: 1
    Result::Ok(())
}
// warning: type parameter `t` should have an upper camel case name: `T`
