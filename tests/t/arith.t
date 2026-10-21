// Integer arithmetic and operator precedence.

fn main() {
    print(1 + 2 * 3);       // expect: 7
    print((1 + 2) * 3);     // expect: 9
    print(10 - 4 - 3);      // expect: 3
    print(17 / 5);          // expect: 3
    print(17 % 5);          // expect: 2
    print(-5 + 2);          // expect: -3
    print(--5);             // expect: 5
    print(1_000_000 * 3);   // expect: 3000000
}
