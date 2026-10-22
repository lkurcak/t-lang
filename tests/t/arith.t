// Integer arithmetic and operator precedence.

fn main() {
    print(1i64 + 2 * 3);     // expect: 7
    print((1i64 + 2) * 3);   // expect: 9
    print(10i64 - 4 - 3);    // expect: 3
    print(17i64 / 5);        // expect: 3
    print(17i64 % 5);        // expect: 2
    print(-5i64 + 2);        // expect: -3
    print(--5i64);           // expect: 5
    print(1_000_000i64 * 3); // expect: 3000000
}
