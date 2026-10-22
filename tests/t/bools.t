// Booleans, comparisons and short-circuiting logic.

fn b(x: bool) -> i64 { if x { 1 } else { 0 } }

fn main() {
    print(b(1i64 < 2));                    // expect: 1
    print(b(2i64 <= 1));                   // expect: 0
    print(b(true && !false));           // expect: 1
    print(b(false || 3i64 == 3));          // expect: 1
    print(b(true != false));            // expect: 1
    print(b(() == ()));                 // expect: 1
    // `||` binds looser than `&&`: this is `true || (false && false)`.
    print(b(true || false && false));   // expect: 1
    // Short-circuit: the division by zero never runs.
    print(b(false && 1i64 / 0 == 0));      // expect: 0
}
