// A nested `fn` shadows an outer one of the same name, and different
// functions, or blocks, can each have their own.
fn helper() -> i64 { 1 }

fn first() -> i64 {
    fn helper() -> i64 { 10 }
    helper()
}

fn second() -> i64 {
    fn helper() -> i64 { 20 }
    helper()
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&helper())?; // expect: 1
    out.line(&first())?;  // expect: 10
    out.line(&second())?; // expect: 20
    {
        // Shadows the top-level `helper` in all of this block, even before.
        out.line(&helper())?; // expect: 30
        fn helper() -> i64 { 30 }
        {
            fn helper() -> i64 { 40 }
            out.line(&helper())?; // expect: 40
        }
        out.line(&helper())?; // expect: 30
    }
    {
        fn helper() -> i64 { 50 }
        out.line(&helper())?; // expect: 50
    }
    out.line(&helper())?; // expect: 1

    // A local declared before the block is shadowed by the block's `fn`,
    // and one declared in the block shadows it.
    let helper: i64 = 60;
    {
        out.line(&helper())?; // expect: 30
        fn helper() -> i64 { 30 }
        let helper: i64 = 70;
        out.line(&helper)?; // expect: 70
    }
    out.line(&helper)?; // expect: 60
    Result::Ok(())
}
