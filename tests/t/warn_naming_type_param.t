// Type parameters are named in UpperCamelCase. The fix renames one only in
// what declares it, though others share its name.
#[derive(Discard)]
struct Wrap<item> { inner: item }

fn unwrap<item>(w: Wrap<item>) -> item {
    let inner: item = w.inner;
    inner
}

fn id<T>(x: T) -> T { x }

fn main() {
    print(unwrap(Wrap { inner: id::<int>(1) }));
}
// warning: type parameter `item` should have an upper camel case name: `Item`
// warning: type parameter `item` should have an upper camel case name: `Item`
// expect: 1
