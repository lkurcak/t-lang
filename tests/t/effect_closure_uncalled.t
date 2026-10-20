// Making a closure doesn't run it: a closure that may panic can be made,
// and thrown away, where nothing may.
fn main() {
    let z = 0;
    let c = || 1 / z;
    print(1); // expect: 1
}
// warning: unused variable `c`
