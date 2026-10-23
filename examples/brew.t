// The Homebrew formula's test (lkurcak/homebrew-tap, Formula/t-lang.rb).
// Keep the two in sync: if this stops compiling, so does `brew test`.

fn fact(n: i64) -> i64 with panic {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn main() with panic {
    print(fact(10)); // expect: 3628800
}
