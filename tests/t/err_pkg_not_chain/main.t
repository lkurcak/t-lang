// For now the packages a program is made of must form a chain, each
// depending on the one before: two that don't know each other could each
// check, and still make proving what implements what go on forever
// together (see docs/26-packages.md).
// error: `left` and `right` don't depend on one another, which packages of one program must, for now
fn main() {
    let _a = left::f() + right::g();
}
