// Booleans, comparisons and short-circuiting logic.

fn b(x: bool) -> int { if x { 1 } else { 0 } }

fn main() {
    print(b(1 < 2));                    // expect: 1
    print(b(2 <= 1));                   // expect: 0
    print(b(true && !false));           // expect: 1
    print(b(false || 3 == 3));          // expect: 1
    print(b(true != false));            // expect: 1
    print(b(() == ()));                 // expect: 1
    // `||` binds looser than `&&`: this is `true || (false && false)`.
    print(b(true || false && false));   // expect: 1
    // Short-circuit: the division by zero never runs.
    print(b(false && 1 / 0 == 0));      // expect: 0
}
