// `t test` runs each `#[test]` of the program, in source order, modules'
// too, and names each by its path. `main` isn't run by it, and a test isn't
// run by `t`: a test that fails here doesn't make the program fail.
fn double(x: i64) -> i64 { x * 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&double(2))?; // expect: 4
    Result::Ok(())
}

#[test]
fn doubles() {
    assert_eq(&double(3), &6);
    assert_ne(&double(3), &7);
    assert(double(0) == 0);
}

mod parsing {
    #[test]
    fn inner() { assert(true); }

    mod deeper {
        #[test]
        fn deepest() { assert_eq(&crate::double(1), &2); }
    }
}

#[test]
fn fails() { assert(super_secret() == 1); }

fn super_secret() -> i64 { 2 }

// test: doubles ... ok
// test: parsing::inner ... ok
// test: parsing::deeper::deepest ... ok
// test: fails ... FAILED: runtime error: panicked: assertion failed
