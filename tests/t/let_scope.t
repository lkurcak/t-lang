// `let`, annotations, shadowing and block scoping.

fn main() {
    let x = 1;
    let y: i64 = x + 1;
    print(y);                 // expect: 2
    let x = x * 10;           // shadows the old x
    print(x);                 // expect: 10
    let z = {
        let x = 100;          // only visible inside this block
        x + 1
    };
    print(z);                 // expect: 101
    print(x);                 // expect: 10
    let b: bool = z > x;      // a different type is fine when shadowing too
    let x = b;
    if x { print(1i64) }         // expect: 1
}
