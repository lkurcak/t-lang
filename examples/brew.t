// The Homebrew formula's test (lkurcak/homebrew-tap, Formula/t-lang.rb):
// the formula installs this file and runs it, expecting `3628800`. So
// changing what it prints breaks `brew test`; changing the language is
// fine, as T's own tests keep this file working.

fn fact(n: i64) -> i64 with panic {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&fact(10))?; // expect: 3628800
    Result::Ok(())
}
