// Effect parameters are named in snake_case, as effects are; the keyword
// says what it is, so an uppercase one works, with a warning.
fn apply<effect E, F: Fn() -> i64 with E + Discard>(f: F) -> i64 with E { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&apply(|| 1))?; // expect: 1
    Result::Ok(())
}
// warning: effect parameter `E` should have a snake case name: `e`
