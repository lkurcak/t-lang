// Each closure captures the one before it, which holds the one before that.
// Each borrows the one before, so each is only one reference big.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let f0 = || 1i64;
    let f1 = || f0() + 1;
    let f2 = || f1() + 1;
    let f3 = || f2() + 1;
    let f4 = || f3() + 1;
    let f5 = || f4() + 1;
    let f6 = || f5() + 1;
    let f7 = || f6() + 1;
    let f8 = || f7() + 1;
    let f9 = || f8() + 1;
    let f10 = || f9() + 1;
    let f11 = || f10() + 1;
    let f12 = || f11() + 1;
    let f13 = || f12() + 1;
    let f14 = || f13() + 1;
    let f15 = || f14() + 1;
    let f16 = || f15() + 1;
    let f17 = || f16() + 1;
    let f18 = || f17() + 1;
    let f19 = || f18() + 1;
    let f20 = || f19() + 1;
    let f21 = || f20() + 1;
    let f22 = || f21() + 1;
    let f23 = || f22() + 1;
    let f24 = || f23() + 1;
    let f25 = || f24() + 1;
    let f26 = || f25() + 1;
    let f27 = || f26() + 1;
    let f28 = || f27() + 1;
    let f29 = || f28() + 1;
    let f30 = || f29() + 1;
    let f31 = || f30() + 1;
    let f32 = || f31() + 1;
    let f33 = || f32() + 1;
    let f34 = || f33() + 1;
    let f35 = || f34() + 1;
    let f36 = || f35() + 1;
    let f37 = || f36() + 1;
    let f38 = || f37() + 1;
    let f39 = || f38() + 1;
    let f40 = || f39() + 1;
    out.line(&f40())?;
    Result::Ok(())
}
// expect: 41
