// Type parameters are named in UpperCamelCase. The fix renames one only in
// what declares it, though others share its name. (One in lowercase is an
// effect parameter.)
#[derive(Discard)]
struct Wrap<My_item> { inner: My_item }

fn unwrap<My_item>(w: Wrap<My_item>) -> My_item {
    let inner: My_item = w.inner;
    inner
}

fn id<T>(x: T) -> T { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&unwrap(Wrap { inner: id::<i64>(1) }))?;
    Result::Ok(())
}
// warning: type parameter `My_item` should have an upper camel case name: `MyItem`
// warning: type parameter `My_item` should have an upper camel case name: `MyItem`
// expect: 1
