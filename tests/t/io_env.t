// The variables of the environment, read through the `Env` in the `Os`:
// by name, or all of them. A compiled program has those of whatever ran
// it too, so this one looks only at those named `T_TEST_`.
use std::env::Env;

fn count(env: Env) -> u32 {
    let mut n = 0u32;
    for v in env.vars() {
        if v.name.starts_with("T_TEST_") {
            n = checked_add(n, 1).unwrap_or(n);
        }
    }
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&os.env.get("T_TEST_HOME").unwrap_or("?"))?;        // expect: /home/ada
    out.line(&os.env.get("T_TEST_EMPTY").unwrap_or("?").len())?; // expect: 0
    out.line(&os.env.get("T_TEST_EQ").unwrap_or("?"))?;          // expect: a=b
    out.line(&os.env.get("T_TEST_MISSING").is_none())?;          // expect: true
    out.line(&count(os.env))                                     // expect: 3
}
// env: T_TEST_HOME=/home/ada
// env: T_TEST_EMPTY=
// env: T_TEST_EQ=a=b
