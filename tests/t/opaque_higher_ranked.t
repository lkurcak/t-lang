// A bound with references in it is for every lifetime, so the closure
// returned for it is too.
fn same() -> impl Fn(&int) -> &int + Discard { |x| x }

fn main() {
    let f = same();
    let a = 1;
    print(*f(&a));                  // expect: 1
    {
        let b = 2;
        print(*f(&b));              // expect: 2
    }
}
