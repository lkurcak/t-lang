//! The math functions of `Float`, correctly rounded: each gives the float
//! nearest the exact value, ties to even, for every input (see
//! docs/102-math.md).
//!
//! Each is worked out first on a *fast path*, in double-double: a value as
//! the sum `hi + lo` of two `f64`s, which carries about 106 bits. What it
//! gives is within a bound the function states of the exact value, so if
//! every value that close rounds to the same float, that's the answer
//! (lemma M2). Else, about once in a million calls, the *slow path* in
//! `fix` works it out again, to hundreds of bits.
//!
//! An `f32`'s function is its `f64`'s fast path, rounded to an `f32`: far
//! more precise than it needs, which an `f32` of its own may be made
//! faster with, later, giving the same results.

use crate::num::{Float, wrap, wrapping_sub};
use crate::option::Option;
use super::dec::{f32_format, f64_format};
use super::fix;

/// `a + b` exactly, as `s + e`: Knuth's two-sum.
fn two_sum(a: f64, b: f64) -> (f64, f64) with {} {
    let s = a + b;
    let bb = s - a;
    (s, a - (s - bb) + (b - bb))
}

/// `a + b` exactly, as `s + e`, of an `|a|` at least `|b|`, or an `a` of 0:
/// Dekker's fast two-sum.
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) with {} {
    let s = a + b;
    (s, b - (s - a))
}

/// `a` as `hi + lo`, each of at most 26 bits: Veltkamp's split, of an `|a|`
/// below 2^996.
fn split(a: f64) -> (f64, f64) with {} {
    let c = 134217729.0 * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// `a · b` exactly, as `p + e`: Dekker's product, of factors whose product
/// and halves neither overflow nor underflow.
fn two_prod(a: f64, b: f64) -> (f64, f64) with {} {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    (p, ah * bh - p + ah * bl + al * bh + al * bl)
}

/// `|x|`.
fn abs(x: f64) -> f64 with {} {
    if x < 0.0 { -x } else { x }
}

/// 2^m.
fn two_to(m: { m: i32 | m >= -1022 && m <= 1023 }) -> f64 with {} {
    f64::from_bits((m + 1023) as u64 << 52)
}

/// `y · 2^m`, exactly, of a `y` from 1/2 to 2 and an `m` from -1021 up:
/// normal, or an infinity if it's past the greatest, which is then the
/// nearest of `y · 2^m` too.
fn scale(y: f64, m: i32) -> f64 with {} {
    if m > 1023 {
        let rest = if m - 1023 > 1023 { 1023 } else { m - 1023 };
        y * two_to(1023) * two_to(rest)
    } else if m < -1021 {
        y * two_to(-1021)
    } else {
        y * two_to(m)
    }
}

/// The `f64` nearest a value that `hi + lo` is within `eps · |hi|` of, if
/// the ends of that interval round to the same `f64`: rounding is
/// monotone, so the value does too (lemma M2). The ends are worked out as
/// `hi + (lo ± err)`, whose two roundings move them by far less than
/// `eps`'s margin. Of a normal value only: a subnormal rounds at fewer
/// bits.
fn decide64(hi: f64, lo: f64, eps: f64) -> Option<f64> with {} {
    let err = abs(hi) * eps;
    let a = hi + (lo - err);
    let b = hi + (lo + err);
    if a == b { Option::Some(a) } else { Option::None }
}

/// As `decide64`, of a value that `hi + lo` is within `err` of.
fn decide64_abs(hi: f64, lo: f64, err: f64) -> Option<f64> with {} {
    let a = hi + (lo - err);
    let b = hi + (lo + err);
    if a == b { Option::Some(a) } else { Option::None }
}

/// `(ah + al) · (bh + bl)`, as `hi + lo`, within 2^-102 of it.
fn dd_mul(ah: f64, al: f64, bh: f64, bl: f64) -> (f64, f64) with {} {
    let (p, e) = two_prod(ah, bh);
    fast_two_sum(p, e + (ah * bl + al * bh))
}

/// The `f32` nearest a value that `hi + lo` is within `eps · |hi|` of, if
/// that's enough to tell. Their sum `y` is rounded, which moves it at most
/// `2^-53 · |y|` more; and the ends, `y ± e`, are each rounded to an `f64`
/// too, which may move them that much again, inward; so `e` is wider by
/// both: then each end in `f64` is past the true one, and rounding it to
/// an `f32` once is monotone (lemma M2).
fn decide32(hi: f64, lo: f64, eps: f64) -> Option<f32> with {} {
    let (a, b) = ends32(hi, lo, eps);
    if a.to_bits() == b.to_bits() { Option::Some(a) } else { Option::None }
}

/// The ends `decide32` rounds: the `f32`s nearest `y ± e`, of `y = hi + lo`.
fn ends32(hi: f64, lo: f64, eps: f64) -> (f32, f32) with {} {
    let y = hi + lo;
    let e = abs(y) * (eps + 2.220446049250313e-16);
    (f32::nearest_f64(y - e), f32::nearest_f64(y + e))
}

/// 2^(j/64), j from 0 to 63, each as `hi + lo`, the bits of each `f64`
/// (scripts/math/tables.c `exp2`).
const EXP2: [u64; 128] = [
    0x3FF0000000000000,
    0x0000000000000000,
    0x3FF02C9A3E778061,
    0xBC719083535B085D,
    0x3FF059B0D3158574,
    0x3C8D73E2A475B465,
    0x3FF0874518759BC8,
    0x3C6186BE4BB284FF,
    0x3FF0B5586CF9890F,
    0x3C98A62E4ADC610B,
    0x3FF0E3EC32D3D1A2,
    0x3C403A1727C57B53,
    0x3FF11301D0125B51,
    0xBC96C51039449B3A,
    0x3FF1429AAEA92DE0,
    0xBC932FBF9AF1369E,
    0x3FF172B83C7D517B,
    0xBC819041B9D78A76,
    0x3FF1A35BEB6FCB75,
    0x3C8E5B4C7B4968E4,
    0x3FF1D4873168B9AA,
    0x3C9E016E00A2643C,
    0x3FF2063B88628CD6,
    0x3C8DC775814A8495,
    0x3FF2387A6E756238,
    0x3C99B07EB6C70573,
    0x3FF26B4565E27CDD,
    0x3C82BD339940E9D9,
    0x3FF29E9DF51FDEE1,
    0x3C8612E8AFAD1255,
    0x3FF2D285A6E4030B,
    0x3C90024754DB41D5,
    0x3FF306FE0A31B715,
    0x3C86F46AD23182E4,
    0x3FF33C08B26416FF,
    0x3C932721843659A6,
    0x3FF371A7373AA9CB,
    0xBC963AEABF42EAE2,
    0x3FF3A7DB34E59FF7,
    0xBC75E436D661F5E3,
    0x3FF3DEA64C123422,
    0x3C8ADA0911F09EBC,
    0x3FF4160A21F72E2A,
    0xBC5EF3691C309278,
    0x3FF44E086061892D,
    0x3C489B7A04EF80D0,
    0x3FF486A2B5C13CD0,
    0x3C73C1A3B69062F0,
    0x3FF4BFDAD5362A27,
    0x3C7D4397AFEC42E2,
    0x3FF4F9B2769D2CA7,
    0xBC94B309D25957E3,
    0x3FF5342B569D4F82,
    0xBC807ABE1DB13CAD,
    0x3FF56F4736B527DA,
    0x3C99BB2C011D93AD,
    0x3FF5AB07DD485429,
    0x3C96324C054647AD,
    0x3FF5E76F15AD2148,
    0x3C9BA6F93080E65E,
    0x3FF6247EB03A5585,
    0xBC9383C17E40B497,
    0x3FF6623882552225,
    0xBC9BB60987591C34,
    0x3FF6A09E667F3BCD,
    0xBC9BDD3413B26456,
    0x3FF6DFB23C651A2F,
    0xBC6BBE3A683C88AB,
    0x3FF71F75E8EC5F74,
    0xBC816E4786887A99,
    0x3FF75FEB564267C9,
    0xBC90245957316DD3,
    0x3FF7A11473EB0187,
    0xBC841577EE04992F,
    0x3FF7E2F336CF4E62,
    0x3C705D02BA15797E,
    0x3FF82589994CCE13,
    0xBC9D4C1DD41532D8,
    0x3FF868D99B4492ED,
    0xBC9FC6F89BD4F6BA,
    0x3FF8ACE5422AA0DB,
    0x3C96E9F156864B27,
    0x3FF8F1AE99157736,
    0x3C85CC13A2E3976C,
    0x3FF93737B0CDC5E5,
    0xBC675FC781B57EBC,
    0x3FF97D829FDE4E50,
    0xBC9D185B7C1B85D1,
    0x3FF9C49182A3F090,
    0x3C7C7C46B071F2BE,
    0x3FFA0C667B5DE565,
    0xBC9359495D1CD533,
    0x3FFA5503B23E255D,
    0xBC9D2F6EDB8D41E1,
    0x3FFA9E6B5579FDBF,
    0x3C90FAC90EF7FD31,
    0x3FFAE89F995AD3AD,
    0x3C97A1CD345DCC81,
    0x3FFB33A2B84F15FB,
    0xBC62805E3084D708,
    0x3FFB7F76F2FB5E47,
    0xBC75584F7E54AC3B,
    0x3FFBCC1E904BC1D2,
    0x3C823DD07A2D9E84,
    0x3FFC199BDD85529C,
    0x3C811065895048DD,
    0x3FFC67F12E57D14B,
    0x3C92884DFF483CAD,
    0x3FFCB720DCEF9069,
    0x3C7503CBD1E949DB,
    0x3FFD072D4A07897C,
    0xBC9CBC3743797A9C,
    0x3FFD5818DCFBA487,
    0x3C82ED02D75B3707,
    0x3FFDA9E603DB3285,
    0x3C9C2300696DB532,
    0x3FFDFC97337B9B5F,
    0xBC91A5CD4F184B5C,
    0x3FFE502EE78B3FF6,
    0x3C839E8980A9CC8F,
    0x3FFEA4AFA2A490DA,
    0xBC9E9C23179C2893,
    0x3FFEFA1BEE615A27,
    0x3C9DC7F486A4B6B0,
    0x3FFF50765B6E4540,
    0x3C99D3E12DD8A18B,
    0x3FFFA7C1819E90D8,
    0x3C874853F3A5931E,
];

/// `1.5 · 2^52`: added to an `f64` of magnitude below 2^51, it rounds it
/// to an integer, which the sum's lowest bits are, in two's complement.
const SHIFTER: f64 = 6755399441055744.0;

/// e^x, for `exp` and the functions made of it: `(hi + lo) · 2^m`, within
/// `EXP_EPS · hi` of it, of an `|x|` below 746 and at least 2^-54. `x` is
/// `(64m + j) · ln 2 / 64 + r`, `64m + j` the integer nearest `64x / ln 2`,
/// so `|r|` is at most a little over `ln 2 / 128`, below 2^-7.5, and `e^x =
/// 2^m · 2^(j/64) · e^r`. Of `ln 2 / 64`, `L_HI` has 36 bits, so its
/// product with the integer, of at most 17, is exact, and so is taking it
/// from `x`, which it's within a factor of two of (Sterbenz); with
/// `L_LO`, `r` is known to about 2^-83. `e^r` is `1 + r + r^2/2`, in
/// double-double, plus `r^3 (1/6 + r/24 + .. + r^5/40320)`, below 2^-25,
/// in `f64`, which leaves out less than 2^-85. All told, within 2^-76 of
/// `e^x` (lemma M1); `EXP_EPS` is 16 times that.
fn exp_core(x: f64) -> (f64, f64, { m: i32 | m >= -33554433 && m <= 33554432 }) with {} {
    exp_core2(x, 0.0)
}

/// e^(x + xl), as `exp_core`, of an `xl` at most 2^-40 of `x`: added to
/// `r`, which is then made a double-double again, its low part at most
/// half the last bit of its high one, as the terms of `e^r` assume.
fn exp_core2(x: f64, xl: f64) -> (f64, f64, { m: i32 | m >= -33554433 && m <= 33554432 }) with {} {
    let t = x * 92.332482616893657 + SHIFTER;
    let kf = t - SHIFTER;
    let bits = t.to_bits();
    let k: i32 = wrap(bits);
    let j = (bits % 64) as usize;
    let rh0 = x - kf * 0.010830424696223417;
    let (ph, pl) = two_prod(kf, 2.5728046223276691e-14);
    let (rh1, rl0) = two_sum(rh0, -ph);
    let (rh, rl) = two_sum(rh1, rl0 - pl + xl);
    let (sh, sl) = two_prod(rh, rh);
    let mut q = 2.4801587301587302e-5;
    q = q * rh + 0.00019841269841269841;
    q = q * rh + 0.0013888888888888889;
    q = q * rh + 0.0083333333333333332;
    q = q * rh + 0.041666666666666664;
    q = q * rh + 0.16666666666666666;
    q = rh * sh * q;
    let (a, b) = fast_two_sum(1.0, rh);
    let (a2, b2) = fast_two_sum(a, 0.5 * sh);
    let lo = b + b2 + (rl + (0.5 * sl + (rl * rh + q)));
    let (eh, el) = fast_two_sum(a2, lo);
    let th = f64::from_bits(EXP2[2 * j]);
    let tl = f64::from_bits(EXP2[2 * j + 1]);
    let (p, e) = two_prod(th, eh);
    let (hi, lo2) = fast_two_sum(p, e + (th * el + tl * eh));
    (hi, lo2, ((k as i64 - j as i64) / 64) as i32)
}

/// 2^-72: `exp_core`'s error, and its margin (lemma M1).
const EXP_EPS: f64 = 2.117582368135751e-22;

/// e^x: 1 of 0, infinity past about 709.78, and 0 below about -745.13.
pub fn exp_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 709.0) {
        if x != x {
            return x + x;
        }
        if x > 710.0 {
            return f64::infinity();
        }
        if x < -746.0 {
            return 0.0;
        }
    }
    if a < 5.551115123125783e-17 {
        return 1.0;
    }
    let (hi, lo, m) = exp_core(x);
    if m > -1022 {
        match decide64(hi, lo, EXP_EPS) {
            Option::Some(y) => return scale(y, m),
            Option::None => {},
        }
    }
    f64::from_bits(fix::exp(x, f64_format()))
}

/// e^x of an `f32`: infinity past about 88.72, 0 below about -103.97.
pub fn exp_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    let a = abs(d);
    if !(a < 88.0) {
        if x != x {
            return x + x;
        }
        if d > 89.0 {
            return f32::infinity();
        }
        if d < -104.0 {
            return 0.0;
        }
    }
    if a < 1.4901161193847656e-8 {
        return 1.0;
    }
    let (hi, lo, m) = exp_core(d);
    match decide32(scale(hi, m), scale(lo, m), EXP_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::exp(d, f32_format()))),
    }
}

/// For `ln`, by the top 7 bits of a significand `m`, `i`: `c`, about 1 over
/// the middle of `m`'s interval, `[1 + i/128, 1 + (i + 1)/128)`, or of
/// half that from `i = 53` on, where `m` is above about sqrt(2) and is
/// halved; and `-ln c`, as `hi + lo`, as bits (scripts/math/tables.c `ln1`). The
/// intervals holding 1 have `c = 1`, so near 1 nothing cancels.
const LN1: [u64; 384] = [
    0x3FF0000000000000,
    0x8000000000000000,
    0x0000000000000000,
    0x3FEFA11CAA01FA12,
    0x3F87DC475F810A69,
    0x3C274944BC161072,
    0x3FEF6310ACA0DBB5,
    0x3F93CEA44346A584,
    0xBC2865AD48159D00,
    0x3FEF25F644230AB5,
    0x3F9B9FC027AF919A,
    0xBC390AE69229DC86,
    0x3FEEE9C7F8458E02,
    0x3FA1B0D98923D97F,
    0xBC474D7444DD6241,
    0x3FEEAE807ABA01EB,
    0x3FA58A5BAFC8E4D3,
    0xBBFCAB8569C56E40,
    0x3FEE741AA59750E4,
    0x3FA95C830EC8E3F2,
    0x3C3EB41D00A417E9,
    0x3FEE3A9179DC1A73,
    0x3FAD276B8ADB0B56,
    0x3C4078F14C95FF53,
    0x3FEE01E01E01E01E,
    0x3FB075983598E471,
    0x3C5006D2999E22DC,
    0x3FEDCA01DCA01DCA,
    0x3FB253F62F0A1417,
    0x3C21F6D34E01D981,
    0x3FED92F2231E7F8A,
    0x3FB42EDCBEA646EE,
    0xBC5511583653349B,
    0x3FED5CAC807572B2,
    0x3FB60658A93750C4,
    0xBC4F108B1D8436D3,
    0x3FED272CA3FC5B1A,
    0x3FB7DA766D7B12D0,
    0x3C4A2240644D7DA2,
    0x3FECF26E5C44BFC6,
    0x3FB9AB42462033AE,
    0xBC4A099E1C184E8E,
    0x3FECBE6D9601CBE7,
    0x3FBB78C82BB0EDA0,
    0xBC53EF0E61F9B03C,
    0x3FEC8B265AFB8A42,
    0x3FBD4313D66CB35D,
    0x3C5B90DD951D90FA,
    0x3FEC5894D10D4986,
    0x3FBF0A30C01162A4,
    0x3C48BE64B8B7759B,
    0x3FEC26B5392EA01C,
    0x3FC0671512CA596F,
    0xBC52F39B81479B67,
    0x3FEBF583EE868D8B,
    0x3FC14785846742AC,
    0x3C394409F1D3F83A,
    0x3FEBC4FD65883E7B,
    0x3FC2266F190A5ACD,
    0xBC6DAB840E7F6177,
    0x3FEB951E2B18FF23,
    0x3FC303D718E47FD5,
    0xBC6B5AE71F658247,
    0x3FEB65E2E3BEEE05,
    0x3FC3DFC2B0ECC62A,
    0x3C6BA62B8C13F7F4,
    0x3FEB37484AD806CE,
    0x3FC4BA36F39A55E5,
    0xBC6F767E433C98AA,
    0x3FEB094B31D922A4,
    0x3FC59338D9982085,
    0x3C68D16EAABA9419,
    0x3FEADBE87F94905E,
    0x3FC66ACD4272AD51,
    0xBC49201C9C3D5165,
    0x3FEAAF1D2F87EBFD,
    0x3FC740F8F54037A3,
    0x3C56D9BF9D57B326,
    0x3FEA82E65130E159,
    0x3FC815C0A14357E9,
    0x3C5141B7F8C5FA9E,
    0x3FEA574107688A4A,
    0x3FC8E928DE886D41,
    0x3C42589EB96A6240,
    0x3FEA2C2A87C51CA0,
    0x3FC9BB362E7DFB85,
    0xBC551439C1FF83E7,
    0x3FEA01A01A01A01A,
    0x3FCA8BECFC882F19,
    0xBC5A8C37918C39EB,
    0x3FE9D79F176B682D,
    0x3FCB5B519E8FB5A6,
    0xBC6D5D8023E61E5F,
    0x3FE9AE24EA5510DA,
    0x3FCC2968558C18C2,
    0x3C36108E3AE024AC,
    0x3FE9852F0D8EC0FF,
    0x3FCCF6354E09C5DD,
    0x3C6339A07D55B696,
    0x3FE95CBB0BE377AE,
    0x3FCDC1BCA0ABEC7B,
    0x3C5C698A33316DFB,
    0x3FE934C67F9B2CE6,
    0x3FCE8C0252AA5A60,
    0xBC3DC074737F9135,
    0x3FE90D4F120190D5,
    0x3FCF550A564B7B37,
    0xBC613A09202FE73D,
    0x3FE8E6527AF1373F,
    0x3FD00E6C45AD501D,
    0xBC63B9568FF6FEAD,
    0x3FE8BFCE8062FF3A,
    0x3FD071B85FCD590D,
    0x3C608B83FCBDEF40,
    0x3FE899C0F601899C,
    0x3FD0D46B579AB74B,
    0x3C721F640E1E5EC9,
    0x3FE87427BCC092B9,
    0x3FD136870293A8B0,
    0x3C686CC531DBA494,
    0x3FE84F00C2780614,
    0x3FD1980D2DD4236F,
    0xBC702C2E4F1B2EB9,
    0x3FE82A4A0182A4A0,
    0x3FD1F8FF9E48A2F3,
    0xBC693FBF3418960D,
    0x3FE8060180601806,
    0x3FD2596010DF763A,
    0xBC49EED8AE0EBD3C,
    0x3FE7E225515A4F1D,
    0x3FD2B9303AB89D25,
    0xBC585AD7F614AB51,
    0x3FE7BEB3922E017C,
    0x3FD31871C9544185,
    0xBC6EA3598981366F,
    0x3FE79BAA6BB6398B,
    0x3FD3772662BFD85C,
    0x3C602A7589FBA088,
    0x3FE77908119AC60D,
    0x3FD3D54FA5C1F710,
    0x3C553668E578D9CD,
    0x3FE756CAC201756D,
    0x3FD432EF2A04E813,
    0xBC683262E2B59206,
    0x3FE734F0C541FE8D,
    0x3FD49006804009D0,
    0xBC4BFF0D07C5DF6D,
    0x3FE713786D9C7C09,
    0x3FD4EC9732600269,
    0xBC71AA87D977DC5E,
    0x3FE6F26016F26017,
    0x3FD548A2C3ADD263,
    0xBC758CE7BF1846EE,
    0x3FE6D1A62681C861,
    0x3FD5A42AB0F4CFE2,
    0xBC7C6BCB7DEE9A3D,
    0x3FE6B1490AA31A3D,
    0x3FD5FF3070A793D4,
    0xBC7063077D7E37B7,
    0x3FF691473A88D0C0,
    0xBFD602D08AF091EC,
    0xBC7A45DB7CFD9230,
    0x3FF6719F3601671A,
    0xBFD5A8CADBBEDFA1,
    0xBC364F5081307F22,
    0x3FF6524F853B4AA3,
    0xBFD54F431B7BE1A8,
    0x3C50B3F6EF6AE452,
    0x3FF63356B88AC0DE,
    0xBFD4F637EBBA9810,
    0x3C768CB3124B9245,
    0x3FF614B36831AE94,
    0xBFD49DA7F3BCC420,
    0x3C6D964A168CCACB,
    0x3FF5F66434292DFC,
    0xBFD44591E0539F49,
    0xBC4A76D6DC2782DA,
    0x3FF5D867C3ECE2A5,
    0xBFD3EDF463C1683E,
    0x3C6C852FE587DEF8,
    0x3FF5BABCC647FA91,
    0xBFD396CE359BBF53,
    0x3C45C5663663D163,
    0x3FF59D61F123CCAA,
    0xBFD3401E12AECBA0,
    0xBC6F95523ADC5C9F,
    0x3FF5805601580560,
    0xBFD2E9E2BCE12286,
    0x3C6F3ED72E23E134,
    0x3FF56397BA7C52E2,
    0xBFD2941AFB186B7C,
    0xBC46A4678EBAA300,
    0x3FF54725E6BB82FE,
    0xBFD23EC5991EBA49,
    0xBC276EBA35BBF0DF,
    0x3FF52AFF56A8054B,
    0xBFD1E9E1678899F5,
    0xBC564B0DD2687939,
    0x3FF50F22E111C4C5,
    0xBFD1956D3B9BC2F9,
    0xBC50E75A3542856F,
    0x3FF4F38F62DD4C9B,
    0xBFD14167EF367784,
    0xBC7EF824DAAF53E9,
    0x3FF4D843BEDC2C4C,
    0xBFD0EDD060B78082,
    0xBC62D4B610D7D4F5,
    0x3FF4BD3EDDA68FE1,
    0xBFD09AA572E6C6D4,
    0xBC7F9E17343426A9,
    0x3FF4A27FAD76014A,
    0xBFD047E60CDE83B7,
    0xBC708869CBF9E344,
    0x3FF4880522014880,
    0xBFCFEB2233EA07CB,
    0xBC28DE00938B4C30,
    0x3FF46DCE34596066,
    0xBFCF474B134DF228,
    0x3C39F1DF7B5DAAB7,
    0x3FF453D9E2C776CA,
    0xBFCEA4449F04AAF5,
    0x3C6F33919AB94074,
    0x3FF43A2730ABEE4D,
    0xBFCE020CC6235AB5,
    0x3C6F0ADB91423F18,
    0x3FF420B5265E5951,
    0xBFCD60A17F903514,
    0x3C650DF841A71B7A,
    0x3FF40782D10E6566,
    0xBFCCC000C9DB3C52,
    0xBC567A2A8500729E,
    0x3FF3EE8F42A5AF07,
    0xBFCC2028AB17F9B5,
    0xBC6C11AA3853A5F0,
    0x3FF3D5D991AA75C6,
    0xBFCB811730B823D4,
    0x3C5D7C46328983C6,
    0x3FF3BD60D9232955,
    0xBFCAE2CA6F672BD8,
    0x3C6A4A356155F779,
    0x3FF3A524387AC822,
    0xBFCA454082E6AB03,
    0x3C5E0DF823A3CB3D,
    0x3FF38D22D366088E,
    0xBFC9A8778DEBAA3A,
    0xBC528FBFB0E3F0FC,
    0x3FF3755BD1C945EE,
    0xBFC90C6DB9FCBCDB,
    0x3C5357718D7CA4CF,
    0x3FF35DCE5F9F2AF8,
    0xBFC871213750E994,
    0x3C6A97A0CA115D60,
    0x3FF34679ACE01346,
    0xBFC7D6903CAF5ACD,
    0x3C60B17C301D6E14,
    0x3FF32F5CED6A1DFA,
    0xBFC73CB9074FD14D,
    0x3C6721A000B4CF01,
    0x3FF3187758E9EBB6,
    0xBFC6A399DABBD383,
    0xBC676332BD4B341F,
    0x3FF301C82AC40260,
    0xBFC60B3100B09474,
    0xBC6526CEE0FD7F4A,
    0x3FF2EB4EA1FED14B,
    0xBFC5737CC9018CDD,
    0x3C600B28EF013C72,
    0x3FF2D50A012D50A0,
    0xBFC4DC7B897BC1C7,
    0xBC4B60AE1FF0E82E,
    0x3FF2BEF98E5A3711,
    0xBFC4462B9DC9B3DC,
    0x3C485388D830C709,
    0x3FF2A91C92F3C105,
    0xBFC3B08B6757F2A7,
    0xBC65E1AD9BE0A4CD,
    0x3FF293725BB804A5,
    0xBFC31B994D3A4F86,
    0x3C61238B5EFE0665,
    0x3FF27DFA38A1CE4D,
    0xBFC28753BC11ABA2,
    0x3C67394D9FA33313,
    0x3FF268B37CD60127,
    0xBFC1F3B925F25D44,
    0xBC608B27BE4E6B15,
    0x3FF2539D7E9177B2,
    0xBFC160C8024B27B0,
    0x3C4355BFD870AFEB,
    0x3FF23EB79717605B,
    0xBFC0CE7ECDCCC28B,
    0xBC41B57FEA88DA98,
    0x3FF22A0122A0122A,
    0xBFC03CDC0A51EC0D,
    0xBC619E2D3F8B7D10,
    0x3FF21579804855E6,
    0xBFBF57BC7D9005DB,
    0x3C5D361574FB24E2,
    0x3FF2012012012012,
    0xBFBE3707EE30487B,
    0xBC49399D9AAF3B33,
    0x3FF1ECF43C7FB84C,
    0xBFBD179788219362,
    0x3C5B12841044A96C,
    0x3FF1D8F5672E4ABD,
    0xBFBBF968769FCA18,
    0x3C506E4FB7AF9C69,
    0x3FF1C522FC1CE059,
    0xBFBADC77EE5AEA8E,
    0xBC5D7D8F39BEE658,
    0x3FF1B17C67F2BAE3,
    0xBFB9C0C32D4D254D,
    0x3C5627A0E199F569,
    0x3FF19E0119E0119E,
    0xBFB8A6477A91DC29,
    0x3C53D4190A482421,
    0x3FF18AB083902BDB,
    0xBFB78D02263D82D7,
    0xBC5CBCA5B4FDB87E,
    0x3FF1778A191BD684,
    0xBFB674F089365A78,
    0xBC4CA64E9980E048,
    0x3FF1648D50FC3201,
    0xBFB55E10050E0382,
    0xBC59A0629E3973E4,
    0x3FF151B9A3FDD5C9,
    0xBFB4485E03DBDFB0,
    0xBC53BA349AADBC6D,
    0x3FF13F0E8D344724,
    0xBFB333D7F8183F4A,
    0x3C4ADAA06E211E9E,
    0x3FF12C8B89EDC0AC,
    0xBFB2207B5C7854A1,
    0xBC5B3F0431EFB154,
    0x3FF11A3019A74826,
    0xBFB10E45B3CAE829,
    0xBC59B5ED72E6D974,
    0x3FF107FBBE011080,
    0xBFAFFA6911AB9309,
    0x3C4CD9F1F95C2EF1,
    0x3FF0F5EDFAB325A2,
    0xBFADDA8ADC67EE59,
    0x3C431936790BB3B2,
    0x3FF0E40655826011,
    0xBFABBCEBFC68F424,
    0x3C4CD1862F854848,
    0x3FF0D24456359E3A,
    0xBFA9A187B573DE81,
    0xBBFB13B26F298A6A,
    0x3FF0C0A7868B4171,
    0xBFA788595A3577C8,
    0xBC12F7C4C5B3C8BD,
    0x3FF0AF2F722EECB5,
    0xBFA5715C4C03CEE1,
    0xBC45101DC4EBF91F,
    0x3FF09DDBA6AF8360,
    0xBFA35C8BFAA13069,
    0x3C050830A65543A8,
    0x3FF08CABB37565E2,
    0xBFA149E3E4005A8D,
    0x3C3A9A4168FCEBEB,
    0x3FF07B9F29B8EAE2,
    0xBF9E72BF2813CE6A,
    0x3C38A4BBA6A354FA,
    0x3FF06AB59C7912FB,
    0xBF9A55F548C5C427,
    0xBC2F60D2FC36A0D9,
    0x3FF059EEA0727586,
    0xBF963D6178690BBE,
    0x3C318ED4D357C9DC,
    0x3FF04949CC1664C5,
    0xBF9228FB1FEA2E0A,
    0xBC23284991FE3D5C,
    0x3FF038C6B78247FC,
    0xBF8C317384C75F0D,
    0xBC2806208C04C21F,
    0x3FF02864FC7729E9,
    0xBF841929F968330C,
    0xBC23AAE809B43DD0,
    0x3FF0182436517A37,
    0xBF78121214586B02,
    0x3C1C7D68C0D910F2,
    0x3FF0000000000000,
    0x8000000000000000,
    0x0000000000000000,
];

/// For `ln`, by `j` from -40 to 72: `c`, nearest `1/(1 + j/2^13)`, and `-ln
/// c`, as bits (scripts/math/tables.c `ln2t`). After `LN1`, `u` is from `1 - 2^-8`
/// to `1 + 2^-7`, so `j` is from -33 to 65.
const LN2: [u64; 339] = [
    0x3FF014191F674111,
    0xBF740C8A7478788D,
    0x3C1E20F8FFFE770A,
    0x3FF01397E11A581B,
    0xBF738BEBB07BDB63,
    0x3C15A5ECCBAEDD1E,
    0x3FF01316AAEAF705,
    0xBF730B50F6512EF5,
    0xBC01E911618B6E55,
    0x3FF012957CD85A28,
    0xBF728ABA45B78A0A,
    0x3C1115AEE8B8B1ED,
    0x3FF0121456E1BDF6,
    0xBF720A279E6E09F2,
    0xBBE6922727A9BA43,
    0x3FF0119339065EF8,
    0xBF7189990033D091,
    0x3C04FDF891D93E1A,
    0x3FF01112234579D1,
    0xBF71090E6AC8075A,
    0x3C150C7E92D3A411,
    0x3FF01091159E4B3E,
    0xBF708887DDE9DF54,
    0xBC0A828D69F38412,
    0x3FF0101010101010,
    0xBF70080559588B25,
    0xBC1F96638CF63675,
    0x3FF00F8F129A0535,
    0xBF6F0F0DB9A69013,
    0xBBFCD4492B372176,
    0x3FF00F0E1D3B67B1,
    0xBF6E0E18D032ADBC,
    0xBBBFC26CCCB7DF07,
    0x3FF00E8D2FF374A2,
    0xBF6D0D2BF5D40041,
    0xBBE033E5877978EC,
    0x3FF00E0C4AC1693C,
    0xBF6C0C472A091FEC,
    0xBC0F4D8C72B2E814,
    0x3FF00D8B6DA482CE,
    0xBF6B0B6A6C50B654,
    0x3C00B68026AC2FA1,
    0x3FF00D0A989BFEBF,
    0xBF6A0A95BC297863,
    0xBC07AD8401F0EEDF,
    0x3FF00C89CBA71A8D,
    0xBF6909C919122467,
    0x3C0A5769F1FE93EA,
    0x3FF00C0906C513CF,
    0xBF6809048289860A,
    0x3BE6958F3F3B017B,
    0x3FF00B8849F52835,
    0xBF670847F80E7661,
    0xBC0D524BFA2FD1ED,
    0x3FF00B0795369587,
    0xBF660793791FD9F1,
    0x3C08E5A84382F1CF,
    0x3FF00A86E88899A5,
    0xBF6506E7053CA0B0,
    0x3B950FF38642A3AF,
    0x3FF00A0643EA7288,
    0xBF6406429BE3C812,
    0xBBFBD84DD2DE6CD8,
    0x3FF00985A75B5E40,
    0xBF6305A63C94570D,
    0x3C0F3C7F636D2B2D,
    0x3FF0090512DA9AF7,
    0xBF620511E6CD6419,
    0xBBDC5C095A5F4345,
    0x3FF00884866766EF,
    0xBF6104859A0E1140,
    0x3C03E99E35A608DF,
    0x3FF0080402010080,
    0xBF60040155D5881E,
    0x3BE8F98E1113F503,
    0x3FF0078385A6A61E,
    0xBF5E070A33460BCD,
    0xBBE1B6EA92059162,
    0x3FF0070311579652,
    0xBF5C0621C9EB9ADF,
    0x3BF8F558679208D5,
    0x3FF00682A5130FBE,
    0xBF5A05496E9A5E65,
    0x3BFDD98CA8DC5A7D,
    0x3FF0060240D8511E,
    0xBF580481205116A7,
    0xBBF9CF9D61BCAED9,
    0x3FF00581E4A69945,
    0xBF5603C8DE0E9755,
    0xBBEF3610848C674C,
    0x3FF00501907D271C,
    0xBF540320A6D1C38E,
    0xBBEB31F3F686CE74,
    0x3FF00481445B39A8,
    0xBF5202887999A5E9,
    0x3BFB055A54100FFD,
    0x3FF0040100401004,
    0xBF50020055655885,
    0xBBE9ABE6BF0FA436,
    0x3FF00380C42AE963,
    0xBF4C031072681A31,
    0xBBB07805A98468B3,
    0x3FF00300901B0511,
    0xBF480240480A21EC,
    0x3BED9A5B8AC9FD19,
    0x3FF00280640FA271,
    0xBF44019029AF8A3A,
    0x3BE03E918549C070,
    0x3FF0020040080100,
    0xBF40010015575489,
    0x3BEC8863F23EF262,
    0x3FF0018024036051,
    0xBF3801201201439F,
    0x3BC8A6861123F9C2,
    0x3FF0010010010010,
    0xBF30008005559549,
    0x3BDDD332A0E20E2F,
    0x3FF0008004002001,
    0xBF20004001555D55,
    0x3BCDDD88863F53F6,
    0x3FF0000000000000,
    0x8000000000000000,
    0x0000000000000000,
    0x3FEFFF0007FFC002,
    0x3F1FFF8002AA9AA9,
    0x3B910E6678AF0AFC,
    0x3FEFFE001FFE0020,
    0x3F2FFF000AAA2A91,
    0x3BA0BBC04DC4E3DD,
    0x3FEFFD0047F940A2,
    0x3F37FEE011FEBB9F,
    0xBBCA8C86145819AB,
    0x3FEFFC007FF00200,
    0x3F3FFE002AA6A911,
    0x3B999E2B62CC6B2D,
    0x3FEFFB00C7E0C4E1,
    0x3F43FE7029A5CA3A,
    0x3BDB2A8740B91D0A,
    0x3FEFFA011FCA0A1E,
    0x3F47FDC047F5E1EE,
    0x3BC0168EA0B7F4E0,
    0x3FEFF90187AA52BE,
    0x3F4BFCF072429637,
    0xBBD512596A6080DE,
    0x3FEFF801FF801FF8,
    0x3F4FFC00AA8AB118,
    0xBBE0FECBEB9B6CDB,
    0x3FEFF7028749F334,
    0x3F51FD78796663F6,
    0xBBF80424976DB98F,
    0x3FEFF6031F064E08,
    0x3F53FCE0A683A3A7,
    0x3BE027199CD06B7D,
    0x3FEFF503C6B3B23B,
    0x3F55FC38DD9C3383,
    0xBBFD13402E81798D,
    0x3FEFF4047E50A1C3,
    0x3F57FB811FAF18F3,
    0xBBC186491276BFCB,
    0x3FEFF30545DB9EC7,
    0x3F59FAB96DBB3EDF,
    0x3BC75CDC2BA1026C,
    0x3FEFF2061D532B9D,
    0x3F5BF9E1C8BF799E,
    0x3BFC03F267DEC185,
    0x3FEFF10704B5CAC9,
    0x3F5DF8FA31BA88EF,
    0x3BAECA47C52B7DB3,
    0x3FEFF007FC01FF00,
    0x3F5FF802A9AB11E6,
    0x3BFE29E3A153E432,
    0x3FEFEF0903364B28,
    0x3F60FB7D98C7CF73,
    0xBBB21D0E88AECB0B,
    0x3FEFEE0A1A513254,
    0x3F61FAF1E53353C9,
    0x3BF75CBEF6FAC368,
    0x3FEFED0B415137C7,
    0x3F62FA5E3A974563,
    0xBC027972A989187D,
    0x3FEFEC0C7834DEF5,
    0x3F63F9C29972C640,
    0x3C053590790ADEBD,
    0x3FEFEB0DBEFAAB7F,
    0x3F64F91F0244EEF3,
    0x3BE9BF8CF3451E10,
    0x3FEFEA0F15A12139,
    0x3F65F873758CC893,
    0xBC02C1E0AE993600,
    0x3FEFE9107C26C423,
    0x3F66F7BFF3C953BC,
    0x3BB9793B5ACF3A39,
    0x3FEFE811F28A186E,
    0x3F67F7047D798388,
    0xBBE762979B7EC596,
    0x3FEFE71378C9A279,
    0x3F68F641131C4087,
    0x3C0EF37D5BC5654A,
    0x3FEFE6150EE3E6D4,
    0x3F69F575B53065BC,
    0xBBEFD993F15C36B6,
    0x3FEFE516B4D76A3E,
    0x3F6AF4A26434C292,
    0xBBFBE4C9560A710A,
    0x3FEFE4186AA2B1A5,
    0x3F6BF3C720A81ADC,
    0xBC0A0D0C25B98059,
    0x3FEFE31A30444224,
    0x3F6CF2E3EB0928CD,
    0x3C0572FC19289F1A,
    0x3FEFE21C05BAA109,
    0x3F6DF1F8C3D696F1,
    0x3C0F748D58917721,
    0x3FEFE11DEB0453CF,
    0x3F6EF105AB8F0628,
    0x3C0464F037E425C1,
    0x3FEFE01FE01FE020,
    0x3F6FF00AA2B10BA0,
    0x3C02821AD5A6D357,
    0x3FEFDF21E50BCBD6,
    0x3F707783D4DD97E8,
    0xBC19F4DEDE0FE9CA,
    0x3FEFDE23F9C69CF9,
    0x3F70F6FE6095F837,
    0x3BFB49840F4A44BD,
    0x3FEFDD261E4ED9C2,
    0x3F717674F4C0DEB8,
    0x3C1F042E30609769,
    0x3FEFDC2852A30896,
    0x3F71F5E7919D7F02,
    0x3C1D416B0AE55511,
    0x3FEFDB2A96C1B00C,
    0x3F727556376B0543,
    0xBC1FA107BFD1DF36,
    0x3FEFDA2CEAA956E9,
    0x3F72F4C0E668983C,
    0x3BF17B8929287A9B,
    0x3FEFD92F4E58841F,
    0x3F7374279ED559C6,
    0x3C126240CF3106B0,
    0x3FEFD831C1CDBED1,
    0x3F73F38A60F064C7,
    0xBC116C36C8B6BF9D,
    0x3FEFD73445078E52,
    0x3F7472E92CF8CDAF,
    0xBBF3D3A89E19BEA7,
    0x3FEFD636D8047A20,
    0x3F74F244032DA4FD,
    0xBC17575CEDD94DD6,
    0x3FEFD5397AC309EB,
    0x3F75719AE3CDF3B3,
    0xBC1E157C49CE9B4A,
    0x3FEFD43C2D41C590,
    0x3F75F0EDCF18BDD8,
    0x3C1BB9CE1ABDB60F,
    0x3FEFD33EEF7F351D,
    0x3F76703CC54D0076,
    0xBC10CF3CCFBF5122,
    0x3FEFD241C179E0CD,
    0x3F76EF87C6A9B391,
    0x3C06B361EE0BCB64,
    0x3FEFD144A3305109,
    0x3F776ECED36DC9AD,
    0x3C13E691AF12FDCB,
    0x3FEFD04794A10E6A,
    0x3F77EE11EBD82EC4,
    0x3C03C2D23A074505,
    0x3FEFCF4A95CAA1B8,
    0x3F786D511027C8C5,
    0xBBF2AA7B074B3D47,
    0x3FEFCE4DA6AB93E9,
    0x3F78EC8C409B7812,
    0xBBFC3D60E7C8318C,
    0x3FEFCD50C7426E20,
    0x3F796BC37D7217FE,
    0x3C119F0DBEC09F3E,
    0x3FEFCC53F78DB9B2,
    0x3F79EAF6C6EA7C47,
    0x3C1C1D6AA5E18A22,
    0x3FEFCB57378C0020,
    0x3F7A6A261D437417,
    0xBC167A3BA0F7E75D,
    0x3FEFCA5A873BCB19,
    0x3F7AE95180BBC8FD,
    0xBC142BC2AB543EDE,
    0x3FEFC95DE69BA47D,
    0x3F7B6878F1923D6C,
    0x3C1964CA200129B1,
    0x3FEFC86155AA1659,
    0x3F7BE79C70058EBB,
    0x3BF9A6C04043F52D,
    0x3FEFC764D465AAE8,
    0x3F7C66BBFC54749D,
    0xBBFA94C86002C4F6,
    0x3FEFC66862CCEC93,
    0x3F7CE5D796BDA122,
    0xBC09449FA7A0B3A9,
    0x3FEFC56C00DE65F4,
    0x3F7D64EF3F7FBF30,
    0x3BFE99C9569CCF1A,
    0x3FEFC46FAE98A1D1,
    0x3F7DE402F6D97506,
    0x3BCA2DC8588168E3,
    0x3FEFC3736BFA2B1E,
    0x3F7E6312BD096335,
    0xBBFB29130FA8C2DC,
    0x3FEFC27739018CFF,
    0x3F7EE21E924E239D,
    0xBC18CF821C263A70,
    0x3FEFC17B15AD52C5,
    0x3F7F612676E64AEB,
    0x3C18690A6651EE8F,
    0x3FEFC07F01FC07F0,
    0x3F7FE02A6B106799,
    0xBBCE44B7E3711E7F,
    0x3FEFBF82FDEC382E,
    0x3F802F9537858132,
    0x3C2B052B7AD35002,
    0x3FEFBE87097C6F5A,
    0x3F806F13418A4F6A,
    0x3C066AF601582F41,
    0x3FEFBD8B24AB3980,
    0x3F80AE8F53B5DC96,
    0xBC2FC35ACDEB2A8D,
    0x3FEFBC8F4F7722D6,
    0x3F80EE096E2764F8,
    0x3C20799210181D0E,
    0x3FEFBB9389DEB7C4,
    0x3F812D8190FE20F2,
    0xBC05E6242B0A27FF,
    0x3FEFBA97D3E084DD,
    0x3F816CF7BC5946C0,
    0x3C20478DBD40E05F,
    0x3FEFB99C2D7B16E5,
    0x3F81AC6BF05808FB,
    0xBC282A36A1241002,
    0x3FEFB8A096ACFACC,
    0x3F81EBDE2D1997D3,
    0x3C18016E479344EB,
];

/// `ln(1 + t)` of a `t = th + tl` below about 2^-14, as `hi + lo`,
/// normalized, within 2^-84 · |t|: `t - t^2/2` in double-double, `t^2` exactly, and the rest
/// in `f64`, `th^3 (1/3 - th/4 + th^2/5 - th^3/6)`, whose error is below
/// 2^-52 of it, which is below 2^-29 of `t`, and of the terms left out,
/// below 2^-90 of `t`; `tl`'s part, `tl / (1 + th)`, to its first two
/// terms.
fn ln1p_small(th: f64, tl: f64) -> (f64, f64) with {} {
    let (s, se) = two_prod(th, th);
    let mut r = -0.16666666666666666;
    r = r * th + 0.20000000000000001;
    r = -(r * th) + 0.25;
    r = -(r * th) + 0.33333333333333331;
    r = th * s * r;
    let (h, l) = fast_two_sum(th, -0.5 * s);
    fast_two_sum(h, l + (tl - tl * th - 0.5 * se + r))
}

/// `ln 2` as `LN2_HI + LN2_LO`, `LN2_HI` of 42 bits, so its product with an
/// exponent, of at most 11, is exact.
const LN2_HI: f64 = 0.69314718055989033;
const LN2_LO: f64 = 5.4979230187083712e-14;

/// ln x, as `hi + lo`, within `LN_EPS · |hi|` of it, of a positive finite
/// `x`. `x` is `2^e · m`, `m` in `[1, 2)`, halved, and `e` one more, where
/// it's above about sqrt(2), so in `[0.7, 1.42)`; a subnormal is scaled up
/// by 2^54 first. Then `u = m · c1`, exactly, as a double-double, is within
/// 2^-7 of 1, and `t = u · c2 - 1` within 2^-14 of 0, and `ln x = e · ln 2
/// - ln c1 - ln c2 + ln(1 + t)`. Where `x` is near 1, `c1` is 1, and near
/// it `c2` too, so `ln x` is `ln(1 + t)` alone, and nothing cancels; and
/// elsewhere the sum is at least a quarter of its largest term. So within
/// 2^-84 of `ln(1 + t)`, plus the tables', 2^-106, of a sum no more than
/// four times its value: 2^-81 all told (lemma M1); `LN_EPS` is 8 times
/// that.
fn ln_core(x: f64) -> (f64, f64) with {} {
    let (b, scaled) = if x < 2.2250738585072014e-308 {
        ((x * 18014398509481984.0).to_bits(), 54)
    } else {
        (x.to_bits(), 0)
    };
    let exp = ((b >> 52) % 2048) as i32;
    let i = ((b >> 45) % 128) as usize;
    let m = f64::from_bits((b & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
    let (m1, e) = if i >= 53 { (m * 0.5, exp - 1022 - scaled) } else { (m, exp - 1023 - scaled) };
    let c1 = f64::from_bits(LN1[3 * i]);
    let a1h = f64::from_bits(LN1[3 * i + 1]);
    let a1l = f64::from_bits(LN1[3 * i + 2]);
    let (uh, ul) = two_prod(m1, c1);
    let jf = (uh - 1.0) * 8192.0 + SHIFTER;
    let j0 = fix::clamp(wrap(jf.to_bits()), 72);
    let j = if j0 < -40 { 0 } else { j0 + 40 } as usize;
    let c2 = f64::from_bits(LN2[3 * j]);
    let a2h = f64::from_bits(LN2[3 * j + 1]);
    let a2l = f64::from_bits(LN2[3 * j + 2]);
    let (p, pe) = two_prod(uh, c2);
    let (th, tl) = two_sum(p - 1.0, pe + ul * c2);
    let (h, l) = ln1p_small(th, tl);
    let ef = e as f64;
    let (s1, s1e) = two_sum(ef * LN2_HI, a1h);
    let (s2, s2e) = two_sum(s1, a2h);
    let (s3, s3e) = two_sum(s2, h);
    fast_two_sum(s3, s1e + s2e + s3e + (ef * LN2_LO + a1l + a2l + l))
}

/// 2^-78: `ln_core`'s error, and its margin.
const LN_EPS: f64 = 3.308722450212111e-24;

/// ln x: minus infinity of 0, a NaN below 0, 0 of 1.
pub fn ln_f64(x: f64) -> f64 with {} {
    if !(x > 0.0) || x == f64::infinity() {
        return if x == 0.0 { -f64::infinity() } else if x > 0.0 { x } else { f64::nan() + x };
    }
    let (hi, lo) = ln_core(x);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::ln(x, f64_format())),
    }
}

/// ln x of an `f32`.
pub fn ln_f32(x: f32) -> f32 with {} {
    if !(x > 0.0) || x == f32::infinity() {
        return if x == 0.0 { -f32::infinity() } else if x > 0.0 { x } else { f32::nan() + x };
    }
    let d = x as f64;
    let (hi, lo) = ln_core(d);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::ln(d, f32_format()))),
    }
}

/// log2 x: ln x times log2 e, in double-double, which adds less than
/// 2^-100 to `ln_core`'s error; exactly `k` of 2^k.
pub fn log2_f64(x: f64) -> f64 with {} {
    if !(x > 0.0) || x == f64::infinity() {
        return ln_f64(x);
    }
    let (h, l) = ln_core(x);
    let (hi, lo) = dd_mul(h, l, 1.4426950408889634, 2.0355273740931033e-17);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::log(x, 2, f64_format())),
    }
}

/// log2 x of an `f32`.
pub fn log2_f32(x: f32) -> f32 with {} {
    if !(x > 0.0) || x == f32::infinity() {
        return ln_f32(x);
    }
    let d = x as f64;
    let (h, l) = ln_core(d);
    let (hi, lo) = dd_mul(h, l, 1.4426950408889634, 2.0355273740931033e-17);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::log(d, 2, f32_format()))),
    }
}

/// log10 x: ln x times log10 e; exactly `k` of 10^k.
pub fn log10_f64(x: f64) -> f64 with {} {
    if !(x > 0.0) || x == f64::infinity() {
        return ln_f64(x);
    }
    let (h, l) = ln_core(x);
    let (hi, lo) = dd_mul(h, l, 0.43429448190325182, 1.0983196502167651e-17);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::log(x, 10, f64_format())),
    }
}

/// log10 x of an `f32`.
pub fn log10_f32(x: f32) -> f32 with {} {
    if !(x > 0.0) || x == f32::infinity() {
        return ln_f32(x);
    }
    let d = x as f64;
    let (h, l) = ln_core(d);
    let (hi, lo) = dd_mul(h, l, 0.43429448190325182, 1.0983196502167651e-17);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::log(d, 10, f32_format()))),
    }
}

/// ln(1 + x), as `hi + lo`, within `LN_EPS · |hi|`, of an `x` above -1 and
/// of magnitude at least 2^-53: `ln1p_small` of `x` itself below 2^-15;
/// else `1 + x` is `s + se` exactly, and `ln(s + se) = ln s + ln(1 +
/// se/s)`, the second below 2^-53, so its first term is enough, and its
/// rounding is below 2^-106 of a value of at least 2^-16.
fn ln1p_core(x: f64) -> (f64, f64) with {} {
    if abs(x) < 3.0517578125e-5 {
        return ln1p_small(x, 0.0);
    }
    let (s, se) = two_sum(1.0, x);
    let (h, l) = ln_core(s);
    fast_two_sum(h, l + se / s)
}

/// ln(1 + x): minus infinity of -1, a NaN below it, and `x` itself where
/// it's below 2^-53, as the rest is below half its last bit.
pub fn ln_1p_f64(x: f64) -> f64 with {} {
    if !(x > -1.0) || x == f64::infinity() {
        return if x == -1.0 { -f64::infinity() } else if x > 0.0 { x } else { f64::nan() + x };
    }
    if abs(x) < 1.1102230246251565e-16 {
        return x;
    }
    let (hi, lo) = ln1p_core(x);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::ln_1p(x, f64_format())),
    }
}

/// ln(1 + x) of an `f32`: `x` itself below 2^-24.
pub fn ln_1p_f32(x: f32) -> f32 with {} {
    if !(x > -1.0) || x == f32::infinity() {
        return if x == -1.0 { -f32::infinity() } else if x > 0.0 { x } else { f32::nan() + x };
    }
    let d = x as f64;
    if abs(d) < 5.9604644775390625e-8 {
        return x;
    }
    let (hi, lo) = ln1p_core(d);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::ln_1p(d, f32_format()))),
    }
}

/// `ln 2` as a double-double.
const LN2_H: f64 = 0.69314718055994529;
const LN2_L: f64 = 2.3190468138462996e-17;

/// 2^x: exactly that power of two of an integer, so even of a subnormal
/// one; else `e^(x ln 2)`, `x ln 2` in double-double, within 2^-94 of it.
pub fn exp2_f64(x: f64) -> f64 with {} {
    if !(abs(x) < 1022.0) {
        if x != x {
            return x + x;
        }
        if x >= 1024.0 {
            return f64::infinity();
        }
        if x <= -1075.0 {
            // 2^-1075 is halfway between 0 and the least subnormal: 0 is
            // even.
            return 0.0;
        }
    }
    if abs(x) < 5.551115123125783e-17 {
        return 1.0;
    }
    if x == x.trunc() {
        return power_of_two(x);
    }
    let (th, tl0) = two_prod(x, LN2_H);
    let (hi, lo, m) = exp_core2(th, tl0 + x * LN2_L);
    if m > -1022 {
        match decide64(hi, lo, EXP_EPS) {
            Option::Some(y) => return scale(y, m),
            Option::None => {},
        }
    }
    f64::from_bits(fix::exp2(x, f64_format()))
}

/// 2^x of an `f32`.
pub fn exp2_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    if !(abs(d) < 126.0) {
        if x != x {
            return x + x;
        }
        if d >= 128.0 {
            return f32::infinity();
        }
        if d <= -150.0 {
            return 0.0;
        }
    }
    if abs(d) < 5.551115123125783e-17 {
        return 1.0;
    }
    if d == d.trunc() {
        return f32::nearest_f64(power_of_two(d));
    }
    let (th, tl0) = two_prod(d, LN2_H);
    let (hi, lo, m) = exp_core2(th, tl0 + d * LN2_L);
    match decide32(scale(hi, m), scale(lo, m), EXP_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::exp2(d, f32_format()))),
    }
}

/// 2^x of an integer `x` from -1074 to 1023, exactly: a subnormal's bits
/// are its significand, a single 1.
fn power_of_two(x: f64) -> f64 with {} {
    let k = fix::clamp(fix::nearest_int(x), 1074);
    if k >= -1022 {
        let m = if k > 1023 { 1023 } else { k };
        two_to(m)
    } else {
        // `2^k = 2^-1022 · 2^(k + 1022)`, exact: the product is the
        // subnormal.
        two_to(-1022) * two_to(if k + 1022 < -52 { -52 } else { k + 1022 })
    }
}

/// e^x - 1, as `hi + lo`, and how far it may be from it, of an `x` from
/// -38 to 710 of magnitude at least 2^-54. Where `exp_core`'s `64m + j`
/// is 0, `x` is `r`, and it's `r + r^2/2 + q` without the 1, within 2^-76
/// of it; else `e^x`, scaled, less 1, exactly, within `EXP_EPS · e^x`.
fn exp_m1_core(x: f64) -> (f64, f64, f64) with {} {
    let t = x * 92.332482616893657 + SHIFTER;
    if t == SHIFTER {
        let (sh, sl) = two_prod(x, x);
        let mut q = 2.4801587301587302e-5;
        q = q * x + 0.00019841269841269841;
        q = q * x + 0.0013888888888888889;
        q = q * x + 0.0083333333333333332;
        q = q * x + 0.041666666666666664;
        q = q * x + 0.16666666666666666;
        q = x * sh * q;
        let (h, l) = fast_two_sum(x, 0.5 * sh);
        let (hi, lo) = fast_two_sum(h, l + (0.5 * sl + q));
        return (hi, lo, abs(hi) * EXP_EPS);
    }
    let (eh, el, m) = exp_core(x);
    let big = scale(eh, m);
    let (s, se) = two_sum(big, -1.0);
    let (hi, lo) = fast_two_sum(s, se + scale(el, m));
    (hi, lo, big * EXP_EPS)
}

/// e^x - 1: `x` itself below 2^-54, -1 below -38, where `e^x` is below
/// 2^-54, and infinity past about 709.78.
pub fn exp_m1_f64(x: f64) -> f64 with {} {
    if !(x < 709.0 && x > -38.0) {
        if x != x {
            return x + x;
        }
        if x > 710.0 {
            return f64::infinity();
        }
        if x <= -38.0 {
            return -1.0;
        }
    }
    if abs(x) < 5.551115123125783e-17 {
        return x;
    }
    let (hi, lo, err) = exp_m1_core(x);
    match decide64_abs(hi, lo, err) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::exp_m1(x, f64_format())),
    }
}

/// e^x - 1 of an `f32`: `x` below 2^-24, -1 below -18.
pub fn exp_m1_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    if !(d < 88.0 && d > -18.0) {
        if x != x {
            return x + x;
        }
        if d > 89.0 {
            return f32::infinity();
        }
        if d <= -18.0 {
            return -1.0;
        }
    }
    if abs(d) < 5.9604644775390625e-8 {
        return x;
    }
    let (hi, lo, err) = exp_m1_core(d);
    match decide32(hi, lo, err / abs(hi)) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::exp_m1(d, f32_format()))),
    }
}

/// Is `y` an odd integer? None from 2^53 up is.
fn is_odd(y: f64) -> bool with {} {
    abs(y) < 9007199254740992.0 && y == y.trunc() && y * 0.5 != (y * 0.5).trunc()
}

/// 2^-67: `pow`'s error: `ln_core`'s, at most 2^-78 of `ln x`, times `y`,
/// an error in the exponent of at most `745 · 2^-78`, and `exp_core`'s.
const POW_EPS: f64 = 6.776263578034403e-21;

/// What `pow` is where `x` or `y` is 0, an infinity or a NaN, or `x` is 1,
/// as IEEE 754-2019 says (§9.2.1), the same as C's: 1 of `y = ±0` and of
/// `x = 1`, even where the other is a NaN; else a NaN of a NaN; of `±0`, an
/// infinity where `y` is negative, the sign of `x` kept where `y` is an odd
/// integer; of `±∞` the other way round; of `y = ±∞`, 1 of `x = -1`, and
/// else 0 or infinity as `|x|` is below or above 1. `None` of finite `x`
/// and `y`, not 0 nor 1, for which it's then the general case.
fn pow_special(x: f64, y: f64) -> Option<f64> with {} {
    if y == 0.0 || x == 1.0 {
        return Option::Some(1.0);
    }
    if x != x || y != y {
        return Option::Some(x + y);
    }
    let infinity = f64::infinity();
    if abs(y) == infinity {
        let ax = abs(x);
        return Option::Some(if ax == 1.0 {
            1.0
        } else if (ax < 1.0) == (y < 0.0) {
            infinity
        } else {
            0.0
        });
    }
    if x == 0.0 || abs(x) == infinity {
        let big = (x == 0.0) == (y < 0.0);
        let r = if big { infinity } else { 0.0 };
        return Option::Some(if is_odd(y) && x.is_sign_negative() { -r } else { r });
    }
    Option::None
}

/// x^y, of IEEE 754's `pow`: `pow_special`'s values; a NaN of a negative
/// `x` and a `y` not an integer; of a negative `x` and an integer `y`,
/// `|x|^y`, negative if `y` is odd. Then `pow_exact`, of the values that
/// are floats or halfway between two (lemma M4); else `e^(y ln x)`.
pub fn pow_f64(x: f64, y: f64) -> f64 with {} {
    match pow_special(x, y) {
        Option::Some(r) => return r,
        Option::None => {},
    }
    if x < 0.0 && y != y.trunc() {
        return f64::nan();
    }
    let negative = x < 0.0 && is_odd(y);
    let ax = abs(x);
    match fix::pow_exact(ax, y, negative, f64_format()) {
        Option::Some(b) => return f64::from_bits(b),
        Option::None => {},
    }
    let (lh, ll) = ln_core(ax);
    let (ph, pl) = two_prod(y, lh);
    let sign = if negative { -1.0 } else { 1.0 };
    if ph > 709.8 {
        return sign * f64::infinity();
    }
    if ph < -745.2 {
        return sign * 0.0;
    }
    let (hi, lo, m) = exp_core2(ph, pl + y * ll);
    if m > -1022 {
        match decide64(hi, lo, POW_EPS) {
            Option::Some(r) => return sign * scale(r, m),
            Option::None => {},
        }
    }
    f64::from_bits(fix::pow(ax, y, negative, f64_format()))
}

/// x^y of `f32`s, as of `f64`s.
pub fn pow_f32(x: f32, y: f32) -> f32 with {} {
    let (xd, yd) = (x as f64, y as f64);
    match pow_special(xd, yd) {
        Option::Some(r) => return f32::nearest_f64(r),
        Option::None => {},
    }
    if xd < 0.0 && yd != yd.trunc() {
        return f32::nan();
    }
    let negative = xd < 0.0 && is_odd(yd);
    let ax = abs(xd);
    match fix::pow_exact(ax, yd, negative, f32_format()) {
        Option::Some(b) => return f32::from_bits(wrap(b)),
        Option::None => {},
    }
    let (lh, ll) = ln_core(ax);
    let (ph, pl) = two_prod(yd, lh);
    let sign: f32 = if negative { -1.0 } else { 1.0 };
    if ph > 89.0 {
        return sign * f32::infinity();
    }
    if ph < -104.0 {
        return sign * 0.0;
    }
    let (hi, lo, m) = exp_core2(ph, pl + yd * ll);
    match decide32(scale(hi, m), scale(lo, m), POW_EPS) {
        Option::Some(r) => sign * r,
        Option::None => f32::from_bits(wrap(fix::pow(ax, yd, negative, f32_format()))),
    }
}

/// `(ah + al) / (bh + bl)`, as `hi + lo`, within 2^-100 of it: the
/// quotient of the high parts, then what's left over, exactly, over `bh`.
fn dd_div(ah: f64, al: f64, bh: f64, bl: f64) -> (f64, f64) with {} {
    let q = ah / bh;
    let (p, pe) = two_prod(q, bh);
    fast_two_sum(q, (ah - p - pe + al - q * bl) / bh)
}

/// sinh x of an `|x|` below 1/4, within 2^-80 of it: `x + x^3 Q(x^2)`,
/// `Q = 1/6 + x^2/120 + ..` to `x^16/19!`, which leaves out less than 2^-92
/// of it; `Q`'s first two terms in double-double, as they're what `x^3 Q`
/// needs to 2^-66 of `Q`, and the rest, below 2^-21 of it, in `f64`.
fn sinh_small(x: f64) -> (f64, f64) with {} {
    let (s, se) = two_prod(x, x);
    let mut tail = 8.2206352466243295e-18;
    tail = tail * s + 2.8114572543455206e-15;
    tail = tail * s + 7.6471637318198164e-13;
    tail = tail * s + 1.6059043836821613e-10;
    tail = tail * s + 2.505210838544172e-08;
    tail = tail * s + 2.7557319223985893e-06;
    tail = tail * s + 0.00019841269841269841;
    tail = s * s * tail;
    let (p, pe) = two_prod(s, 0.0083333333333333332);
    let (q, qe) = two_sum(0.16666666666666666, p);
    let ql = qe
        + (9.2518585385429707e-18
            + pe
            + s * 1.1564823173178714e-19
            + se * 0.0083333333333333332
            + tail);
    let (th, tl) = dd_mul(x, 0.0, s, se);
    let (uh, ul) = dd_mul(th, tl, q, ql);
    let (h, l) = fast_two_sum(x, uh);
    (h, l + ul)
}

/// cosh x of an `|x|` below 1/4, within 2^-74 of it: `1 + x^2/2 + x^4 R`,
/// `R = 1/24 + x^2/720 + .. + x^14/18!`, the first term of `R` and its
/// product with `x^4`, up to 2^-12.5, in double-double, the rest of `R`,
/// below 2^-13 of it, in `f64`; normalized, as `dd_div` needs.
fn cosh_small(x: f64) -> (f64, f64) with {} {
    let (s, se) = two_prod(x, x);
    let mut t = 1.5619206968586225e-16;
    t = t * s + 4.7794773323873853e-14;
    t = t * s + 1.1470745597729725e-11;
    t = t * s + 2.08767569878681e-09;
    t = t * s + 2.7557319223985888e-07;
    t = t * s + 2.4801587301587302e-05;
    t = t * s + 0.0013888888888888889;
    t = s * t;
    let (r, re) = two_sum(0.041666666666666664, t);
    let (fh, fl) = dd_mul(s, se, s, se);
    let (ph, pl) = dd_mul(fh, fl, r, re + 2.3129646346357427e-18);
    let (h, l) = fast_two_sum(1.0, 0.5 * s);
    let (h2, l2) = fast_two_sum(h, ph);
    fast_two_sum(h2, l + l2 + (0.5 * se + pl))
}

/// `e^a` and `e^-a` of an `a` from 1/4 to 37, each as `hi + lo`, scaled:
/// within `EXP_EPS` of each, relatively.
fn exp_pair(a: f64) -> (f64, f64, f64, f64) with {} {
    let (ph, pl, pm) = exp_core(a);
    let (mh, ml, mm) = exp_core(-a);
    (scale(ph, pm), scale(pl, pm), scale(mh, mm), scale(ml, mm))
}

/// 2^-69: `sinh`'s error, of `e^a - e^-a` where `a ≥ 1/4`, whose two
/// terms' errors add up to at most `coth(1/4) < 4.1` times that of each,
/// relatively (lemma M1); and `tanh`'s, whose `(e^2a - 1)/(e^2a + 1)`
/// the same way is within `2.6 · EXP_EPS` and a division's 2^-100.
const HYP_EPS: f64 = 1.6940658945086007e-21;

/// sinh x, as `hi + lo` and the scale `m`: the series below 1/4; then
/// `(e^a - e^-a)/2`; then from 37 on, where `e^-2a` is below 2^-106,
/// `e^a / 2`, as `exp_core` gives it, not yet scaled, as it may overflow.
/// Of `a = |x|`, from 2^-26 to 711.
fn sinh_core(a: f64) -> (f64, f64, i32) with {} {
    if a < 0.25 {
        let (h, l) = sinh_small(a);
        return (h, l, 0);
    }
    if a < 37.0 {
        let (ph, pl, mh, ml) = exp_pair(a);
        let (d, de) = two_sum(ph, -mh);
        let (h, l) = fast_two_sum(d, de + (pl - ml));
        return (0.5 * h, 0.5 * l, 0);
    }
    let (h, l, m) = exp_core(a);
    (h, l, m - 1)
}

/// cosh x, as `sinh_core` does sinh, of `a = |x|` from 2^-26 to 711.
fn cosh_core(a: f64) -> (f64, f64, i32) with {} {
    if a < 0.25 {
        let (h, l) = cosh_small(a);
        return (h, l, 0);
    }
    if a < 37.0 {
        let (ph, pl, mh, ml) = exp_pair(a);
        let (d, de) = two_sum(ph, mh);
        let (h, l) = fast_two_sum(d, de + (pl + ml));
        return (0.5 * h, 0.5 * l, 0);
    }
    let (h, l, m) = exp_core(a);
    (h, l, m - 1)
}

/// sinh x: `x` itself below 2^-26, where the rest is below half its last
/// bit, and an infinity past about 710.48.
pub fn sinh_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 711.0) {
        return if x != x { x + x } else if x < 0.0 { -f64::infinity() } else { f64::infinity() };
    }
    if a < 1.4901161193847656e-8 {
        return x;
    }
    let (hi, lo, m) = sinh_core(a);
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    match decide64(hi, lo, HYP_EPS) {
        Option::Some(y) => sign * scale(y, m),
        Option::None => f64::from_bits(fix::sinh(a, x < 0.0, f64_format())),
    }
}

/// sinh x of an `f32`: `x` below 2^-12, infinity past about 89.4.
pub fn sinh_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < 90.0) {
        return if x != x { x + x } else if x < 0.0 { -f32::infinity() } else { f32::infinity() };
    }
    if a < 0.000244140625 {
        return x;
    }
    let (hi, lo, m) = sinh_core(a);
    let sign: f32 = if x < 0.0 { -1.0 } else { 1.0 };
    match decide32(scale(hi, m), scale(lo, m), HYP_EPS) {
        Option::Some(y) => sign * y,
        Option::None => f32::from_bits(wrap(fix::sinh(a, x < 0.0, f32_format()))),
    }
}

/// cosh x: 1 below 2^-26, an infinity past about 710.48.
pub fn cosh_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 711.0) {
        return if x != x { x + x } else { f64::infinity() };
    }
    if a < 1.4901161193847656e-8 {
        return 1.0;
    }
    let (hi, lo, m) = cosh_core(a);
    match decide64(hi, lo, HYP_EPS) {
        Option::Some(y) => scale(y, m),
        Option::None => f64::from_bits(fix::cosh(a, f64_format())),
    }
}

/// cosh x of an `f32`: 1 below 2^-12.
pub fn cosh_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < 90.0) {
        return if x != x { x + x } else { f32::infinity() };
    }
    if a < 0.000244140625 {
        return 1.0;
    }
    let (hi, lo, m) = cosh_core(a);
    match decide32(scale(hi, m), scale(lo, m), HYP_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::cosh(a, f32_format()))),
    }
}

/// tanh x, as `hi + lo`, of `a = |x|` from 2^-27 to 19.1: `sinh / cosh` below
/// 1/4, else `(e^2a - 1)/(e^2a + 1)`.
fn tanh_core(a: f64) -> (f64, f64) with {} {
    if a < 0.25 {
        let (sh, sl) = sinh_small(a);
        let (ch, cl) = cosh_small(a);
        return dd_div(sh, sl, ch, cl);
    }
    let (h, l, m) = exp_core(2.0 * a);
    let (eh, el) = (scale(h, m), scale(l, m));
    let (nh, nl) = two_sum(eh, -1.0);
    let (dh, dl) = two_sum(eh, 1.0);
    dd_div(nh, nl + el, dh, dl + el)
}

/// tanh x: `x` below 2^-27, where `x^3/3` is below half its last bit, and
/// ±1 from 19.1 on, where `1 - tanh x` is below 2^-55.
pub fn tanh_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 19.1) {
        return if x != x { x + x } else if x < 0.0 { -1.0 } else { 1.0 };
    }
    if a < 7.450580596923828e-9 {
        return x;
    }
    let (hi, lo) = tanh_core(a);
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    match decide64(hi, lo, HYP_EPS) {
        Option::Some(y) => sign * y,
        Option::None => f64::from_bits(fix::tanh(a, x < 0.0, f64_format())),
    }
}

/// tanh x of an `f32`: `x` below 2^-12, ±1 from 9.1 on.
pub fn tanh_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < 9.1) {
        return if x != x { x + x } else if x < 0.0 { -1.0 } else { 1.0 };
    }
    if a < 0.000244140625 {
        return x;
    }
    let (hi, lo) = tanh_core(a);
    let sign: f32 = if x < 0.0 { -1.0 } else { 1.0 };
    match decide32(hi, lo, HYP_EPS) {
        Option::Some(y) => sign * y,
        Option::None => f32::from_bits(wrap(fix::tanh(a, x < 0.0, f32_format()))),
    }
}

/// sin(j/64) and cos(j/64), j from 0 to 51, each as `hi + lo`, as bits
/// (scripts/math/tables.c `sincos`).
const SINCOS: [u64; 208] = [
    0x0000000000000000,
    0x0000000000000000,
    0x3FF0000000000000,
    0x0000000000000000,
    0x3F8FFFAAAAEEEED5,
    0xBC02AB639A9F0776,
    0x3FEFFF000155549F,
    0x3C828A28A03A5EF3,
    0x3F9FFEAAAEEEE86F,
    0xBC3CD406FB224AE2,
    0x3FEFFC00155527D3,
    0xBC83B54492D89B5B,
    0x3FA7FDC01032FBA9,
    0xBC4599BDF46E997A,
    0x3FEFF7006BFDF99F,
    0xBC78B3B560648D5F,
    0x3FAFFAAAEEED4EDB,
    0xBC42D16D32684B69,
    0x3FEFF0015549F4D3,
    0x3C8328387B99426F,
    0x3FB3FACB12D1755B,
    0xBC5921915299468B,
    0x3FEFE7034129EF6F,
    0xBC6CBF4337C96F97,
    0x3FB7F701032550E4,
    0x3C3AFC2D1800501A,
    0x3FEFDC06BF7E6B9B,
    0x3C831902B535F8DB,
    0x3FBBF1B78568391D,
    0x3C5E91841DEA4CC8,
    0x3FEFCF0C800E99B1,
    0x3C6EA3D786D186AC,
    0x3FBFEAAEEE86EE36,
    0xBC4AFCB2BCC6F03B,
    0x3FEFC015527D5BD3,
    0x3C8B68F35094EFB8,
    0x3FC1F0D3D7AFCEAF,
    0xBC66EF95099769A5,
    0x3FEFAF22263C4BD3,
    0xBC552ACE133A2769,
    0x3FC3EB312C5D66CB,
    0x3C647D666B66CB91,
    0x3FEF9C340A7CC428,
    0x3C8C5B6B063B7462,
    0x3FC5E44FCFA126F3,
    0xBC66F443063F89B6,
    0x3FEF874C2E1EECF6,
    0xBC8C6514E1332B16,
    0x3FC7DC102FBAF2B5,
    0x3C45AB50E23C97C3,
    0x3FEF706BDF9ECE1C,
    0xBC8698C80C36DCB4,
    0x3FC9D252D0CEC312,
    0x3C59C43D80B1137D,
    0x3FEF57948CFF6797,
    0x3C6E3A0D3E03B1D4,
    0x3FCBC6F84EDC6199,
    0x3C69C1A56A7B0CAB,
    0x3FEF3CC7C3B3D16E,
    0xBC621A3AD28A3494,
    0x3FCDB9E15FB5A5D0,
    0xBC632E20D6CC6FC2,
    0x3FEF20073086649F,
    0x3C7B940416C1984B,
    0x3FCFAAEED4F31577,
    0xBC615D88508E32B8,
    0x3FEF01549F7DEEA1,
    0x3C8D3C1E99E5CAFD,
    0x3FD0CD00CEF36436,
    0xBC79FB0A0C93E2B4,
    0x3FEEE0B1FBC0F11C,
    0xBC4BFD2380BBC3B1,
    0x3FD1C37D64C6B876,
    0x3C746076FE0DCFF4,
    0x3FEEBE214F76EFA8,
    0xBC802F9F12BA543E,
    0x3FD2B8DDC43EB49F,
    0x3C61553899F2D807,
    0x3FEE99A4C3A7CD83,
    0xBC82264B1BC53CE8,
    0x3FD3AD129769D3D8,
    0x3C003D550487839A,
    0x3FEE733EA0193D40,
    0xBC86428B3546CE13,
    0x3FD4A00C9B0F3D20,
    0x3C7823BA6BB08EAD,
    0x3FEE4AF14B2A449C,
    0xBC868CA02E8A6833,
    0x3FD591BC9FA2F597,
    0x3C67C74BAC3FE0CB,
    0x3FEE20BF49ACD6C1,
    0xBC5660AEC7EF636B,
    0x3FD682138A38D7F7,
    0xBC7D889202444AAD,
    0x3FEDF4AB3EBD875E,
    0xBC8E2D8A7E6736C4,
    0x3FD7710255764214,
    0xBC66EAD7314BB6CE,
    0x3FEDC6B7EB995912,
    0x3C54B364776DCD35,
    0x3FD85E7A12826949,
    0x3C78A40E9B5FACE0,
    0x3FED96E82F71A9DC,
    0x3C8FF61BD5D2039D,
    0x3FD94A6BE9F546C5,
    0xBC769CE13E683F58,
    0x3FED653F073E4040,
    0xBC876236434BEC37,
    0x3FDA34C91CC50CCA,
    0xBC5A310E3B50CECD,
    0x3FED31BF8D8D7C06,
    0x3C7E60DD3089CBDD,
    0x3FDB1D8305321617,
    0xBC7AE242CB99F519,
    0x3FECFC6CFA52AD9F,
    0x3C88B5B5508F2A0D,
    0x3FDC048B17B140A3,
    0x3C619FE6757E9FA7,
    0x3FECC54AA2B2972E,
    0x3C64EE162BA83A98,
    0x3FDCE9D2E3D4A51F,
    0xBC62FC8A12DAE298,
    0x3FEC8C5BF8CE1A84,
    0x3C7AB3D1A1590123,
    0x3FDDCD4C15329C9A,
    0x3C70D4C6E171FD9A,
    0x3FEC51A48B8B175E,
    0xBC61BBB43B9AA880,
    0x3FDEAEE8744B05F0,
    0xBC5789B43C9B027D,
    0x3FEC1528065B7D50,
    0xBC8892111312E828,
    0x3FDF8E99E76ABC97,
    0x3C59D950AF2D00A3,
    0x3FEBD6EA310294F5,
    0x3C731BBCC88C109D,
    0x3FE0362939C69955,
    0xBC82D8CD78397B01,
    0x3FEB96EEEF58840E,
    0x3C545A3CC78FADE0,
    0x3FE0A4021E9E1001,
    0xBC86F643A13914F6,
    0x3FEB553A410C104E,
    0x3C58FF7947027A15,
    0x3FE110D0C4B69C3B,
    0x3C8D918998809981,
    0x3FEB11D04162A4C6,
    0x3C71DD561EFBC0C2,
    0x3FE17C8E5F2EEDB0,
    0x3C635E57102E2488,
    0x3FEACCB526F69DE5,
    0x3C88FB6A8DD6B6CC,
    0x3FE1E7343236574C,
    0x3C722A3FA4F41D5A,
    0x3FEA85ED4373E02D,
    0x3C69BE06385EC792,
    0x3FE250BB93788BBB,
    0x3C7EA3D02457BCCE,
    0x3FEA3D7D0352BDCF,
    0xBC868DBAECA19669,
    0x3FE2B91DEA88421E,
    0xBC8FA371DB216AB0,
    0x3FE9F368ED912F85,
    0xBC81D200C5791606,
    0x3FE32054B148BC4F,
    0x3C8F6B42095A135B,
    0x3FE9A7B5A36A6514,
    0x3C8722CFCC9FA7A9,
    0x3FE386597456282B,
    0xBC710FADA93B07A8,
    0x3FE95A67E00CB1FD,
    0xBC80BEFDA21F862D,
    0x3FE3EB25D36CD53A,
    0xBC5BE570E1570FC0,
    0x3FE90B84784DDAF7,
    0xBC70FEB10AB93B87,
    0x3FE44EB381CF386B,
    0xBC83ED6C1E6A5505,
    0x3FE8BB105A5DC900,
    0x3C8863E03E9474C1,
    0x3FE4B0FC46AAB761,
    0x3C20DA05738CC59C,
    0x3FE869108D77A6C6,
    0x3C7338FFE2BFE9DD,
    0x3FE511F9FD7B351C,
    0xBC85C0E861C48831,
    0x3FE8158A31916D5D,
    0xBC6DE8B90B8228DE,
    0x3FE571A6966D59B3,
    0x3C5C843B4D0FB197,
    0x3FE7C0827F09E54F,
    0xBC6C73D6D72AEE68,
    0x3FE5CFFC16BF8F0D,
    0x3C896CB370EB578A,
    0x3FE769FEC655211F,
    0xBC6827D5CF8C68C5,
    0x3FE62CF49921AC79,
    0xBC8EDD9855B6241A,
    0x3FE712046FA77678,
    0x3C8425B0A5029C81,
    0x3FE6888A4E134B2F,
    0xBC86B7D37644D5E6,
    0x3FE6B898FA9EFB5D,
    0x3C715AC786CCF4B2,
    0x3FE6E2B77C40BDE1,
    0xBC70E729857FAD53,
    0x3FE65DC1FDEB8CBA,
    0xBC597C1B47337C77,
];

/// `π/2` as four parts, the first three of 33 bits, so their products with
/// an integer below 2^20 are exact.
const PIO2_1: f64 = 1.5707963267341256;
const PIO2_2: f64 = 6.077100506303966e-11;
const PIO2_3: f64 = 2.0222662487111665e-21;
const PIO2_4: f64 = 8.4784276603688996e-32;

/// `x` less the nearest multiple of π/2, `k`, as `rh + rl`, within 2^-112 of
/// it, and `k` mod 4, of an `|x|` below 2^20, else of `x` from `fix::reduce`
/// at 192 bits; `None` where `|r|` is below 2^-20, where that's too little
/// of it (Cody and Waite's reduction). The first three parts' products are
/// exact, and so is taking the first from `x`, which it's within a factor
/// of two of; the rest is added up in double-double. `π/2` is known to
/// 2^-152 of it, times `k`, at most 2^20.
fn trig_reduce(x: f64) -> Option<(f64, f64, u32)> with {} {
    let (rh, rl, q) = if abs(x) < 1048576.0 {
        let t = x * 0.63661977236758138 + SHIFTER;
        let k = t - SHIFTER;
        let q = (t.to_bits() % 4) as u32;
        let t1 = x - k * PIO2_1;
        let (s1, e1) = two_sum(t1, -k * PIO2_2);
        let (s2, e2) = two_sum(s1, -k * PIO2_3);
        let (p, pe) = two_prod(k, PIO2_4);
        let (s3, e3) = two_sum(s2, -p);
        let (rh, rl) = two_sum(s3, e1 + e2 + e3 - pe);
        (rh, rl, q)
    } else {
        let a = abs(x);
        let (r, q) = fix::reduce(a, 7);
        let rh = r.to_f64();
        let rl = r.sub(&fix::Fix::of(7, rh)).to_f64();
        if x < 0.0 { (-rh, -rl, (4 - q) % 4) } else { (rh, rl, q) }
    };
    if abs(rh) < 9.5367431640625e-7 {
        return Option::None;
    }
    Option::Some((rh, rl, q))
}

/// `sin r` and `cos r`, each as `hi + lo`, within 2^-90 of them, of an
/// `r = rh + rl` at most π/4 and a little: `r = a + b`, `a = j/64` from the
/// table, so `|b| ≤ 1/128`, and `sin r = sin a cos b + cos a sin b`, `cos r =
/// cos a cos b - sin a sin b`, which don't cancel, as `|a| ≥ 2|b|` or `a =
/// 0`. `sin b = b - b^3 Q`, `Q = 1/6 - b^2/120 + ..`, its first term in
/// double-double, as `b^3 Q` must be known to 2^-74 of itself, and the rest,
/// below 2^-18 of it, in `f64`; and `cos b = 1 - b^2/2 + b^4 (1/24 - ..)`,
/// the last below 2^-32.
fn sincos_core(rh: f64, rl: f64) -> (f64, f64, f64, f64) with {} {
    let jf = rh * 64.0 + SHIFTER;
    let j = fix::clamp(wrap(jf.to_bits()), 51);
    let a = (jf - SHIFTER) * 0.015625;
    let (bh, bl) = two_sum(rh - a, rl);
    let (s2h, s2l0) = two_prod(bh, bh);
    let s2l = s2l0 + 2.0 * bh * bl;
    let mut t = -2.505210838544172e-08;
    t = t * s2h + 2.7557319223985893e-06;
    t = -(t * s2h) + 0.00019841269841269841;
    t = -(t * s2h) + 0.0083333333333333332;
    t = s2h * t;
    let (q, qe) = two_sum(0.16666666666666666, -t);
    let (ch3, cl3) = dd_mul(bh, bl, s2h, s2l);
    let (uh, ul) = dd_mul(ch3, cl3, q, qe + 9.2518585385429707e-18);
    let (sbh, sbl0) = two_sum(bh, -uh);
    let sbl = sbl0 + (bl - ul);
    let mut c4 = -2.4801587301587302e-05;
    c4 = c4 * s2h + 0.0013888888888888889;
    c4 = -(c4 * s2h) + 0.041666666666666664;
    c4 = s2h * s2h * c4;
    let (cbh, cbl0) = fast_two_sum(1.0, -0.5 * s2h);
    let cbl = cbl0 + (c4 - 0.5 * s2l);
    if j == 0 {
        return (sbh, sbl, cbh, cbl);
    }
    let i = if j < 0 { -j } else { j } as usize;
    let sah0 = f64::from_bits(SINCOS[4 * i]);
    let sal0 = f64::from_bits(SINCOS[4 * i + 1]);
    let cah = f64::from_bits(SINCOS[4 * i + 2]);
    let cal = f64::from_bits(SINCOS[4 * i + 3]);
    let (sah, sal) = if j < 0 { (-sah0, -sal0) } else { (sah0, sal0) };
    let (p1h, p1l) = dd_mul(sah, sal, cbh, cbl);
    let (p2h, p2l) = dd_mul(cah, cal, sbh, sbl);
    let (sh, se) = two_sum(p1h, p2h);
    let (p3h, p3l) = dd_mul(cah, cal, cbh, cbl);
    let (p4h, p4l) = dd_mul(sah, sal, sbh, sbl);
    let (ch, ce) = two_sum(p3h, -p4h);
    let (s, sl) = fast_two_sum(sh, se + p1l + p2l);
    let (c, cl) = fast_two_sum(ch, ce + p3l - p4l);
    (s, sl, c, cl)
}

/// 2^-74: the trigonometric functions' error, the reduction's and
/// `sincos_core`'s, and of `tan`, a division's, with a margin.
const TRIG_EPS: f64 = 5.293955920339377e-23;

/// sin x, cos x or tan x, as `hi + lo`, of a finite `x` from 2^-27 up:
/// `None` where it's for the slow path.
fn trig_core(x: f64, which: fix::Trig) -> Option<(f64, f64)> with {} {
    let (rh, rl, q) = match trig_reduce(x) {
        Option::Some(r) => r,
        Option::None => return Option::None,
    };
    let (sh, sl, ch, cl) = sincos_core(rh, rl);
    Option::Some(match which {
        fix::Trig::Sin => match q {
            0 => (sh, sl),
            1 => (ch, cl),
            2 => (-sh, -sl),
            _ => (-ch, -cl),
        },
        fix::Trig::Cos => match q {
            0 => (ch, cl),
            1 => (-sh, -sl),
            2 => (-ch, -cl),
            _ => (sh, sl),
        },
        fix::Trig::Tan => {
            if q % 2 == 0 { dd_div(sh, sl, ch, cl) } else { dd_div(-ch, -cl, sh, sl) }
        },
    })
}

/// The `f64` nearest sin x, cos x or tan x.
fn trig_f64(x: f64, which: fix::Trig) -> f64 with {} {
    match trig_core(x, which) {
        Option::Some((hi, lo)) => match decide64(hi, lo, TRIG_EPS) {
            Option::Some(y) => return y,
            Option::None => {},
        },
        Option::None => {},
    }
    f64::from_bits(fix::trig(x, which, f64_format()))
}

/// The `f32` nearest sin x, cos x or tan x.
fn trig_f32(x: f64, which: fix::Trig) -> f32 with {} {
    match trig_core(x, which) {
        Option::Some((hi, lo)) => match decide32(hi, lo, TRIG_EPS) {
            Option::Some(y) => return y,
            Option::None => {},
        },
        Option::None => {},
    }
    f32::from_bits(wrap(fix::trig(x, which, f32_format())))
}

/// sin x: a NaN of an infinity, and `x` itself below 2^-26, where `x^3/6`
/// is below half its last bit.
pub fn sin_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 1.4901161193847656e-8 {
        return x;
    }
    trig_f64(x, fix::Trig::Sin)
}

/// cos x: a NaN of an infinity, and 1 below 2^-27, where `1 - x^2/2` is
/// above the halfway point below 1.
pub fn cos_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 7.450580596923828e-9 {
        return 1.0;
    }
    trig_f64(x, fix::Trig::Cos)
}

/// tan x: a NaN of an infinity, and `x` below 2^-27, where `x^3/3` is below
/// half its last bit.
pub fn tan_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 7.450580596923828e-9 {
        return x;
    }
    trig_f64(x, fix::Trig::Tan)
}

/// sin x of an `f32`: `x` below 2^-13.
pub fn sin_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 0.0001220703125 {
        return x;
    }
    trig_f32(x as f64, fix::Trig::Sin)
}

/// cos x of an `f32`: 1 below 2^-13.
pub fn cos_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 0.0001220703125 {
        return 1.0;
    }
    trig_f32(x as f64, fix::Trig::Cos)
}

/// tan x of an `f32`: `x` below 2^-13.
pub fn tan_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < f64::infinity()) {
        return x - x;
    }
    if a < 0.0001220703125 {
        return x;
    }
    trig_f32(x as f64, fix::Trig::Tan)
}

// The functions of step 103 (docs/103-more-math.md): the inverses of the
// trigonometric and hyperbolic ones, `cbrt` and `hypot`, and `sin_pi` and
// `cos_pi`.

/// π, π/2 and π/4, as double-doubles, within 2^-107 of each.
const PI_H: f64 = 3.1415926535897931;
const PI_L: f64 = 1.2246467991473532e-16;
const PIO2_H: f64 = 1.5707963267948966;
const PIO2_L: f64 = 6.123233995736766e-17;
const PIO4_H: f64 = 0.78539816339744828;
const PIO4_L: f64 = 3.061616997868383e-17;

/// 2^-76: the error the functions of step 103 decide with, but for the
/// inverse hyperbolic ones, which are `ln`'s and decide with `LN_EPS`, and
/// `sin_pi` and `cos_pi`, which are `sin`'s and decide with `TRIG_EPS`. Each
/// says its own, which is below 2^-92 (lemma M1): this is far wider, so
/// that the hardest of random inputs, which the tests carry, go the slow
/// way and check it, at a cost of once in about 2^22 calls.
const ARC_EPS: f64 = 1.3234889800848443e-23;

/// `x` as `m · 2^e`, `m` from 1 to below 2, exactly, of a finite `x` above
/// 0, a subnormal one too, which is scaled up by 2^54 first.
fn parts(x: f64) -> (f64, { e: i32 | e >= -1077 && e <= 1024 }) with {} {
    let (b, s) = if x < 2.2250738585072014e-308 {
        ((x * 18014398509481984.0).to_bits(), 54)
    } else {
        (x.to_bits(), 0)
    };
    let e = ((b >> 52) % 2048) as i32 - 1023 - s;
    (f64::from_bits((b & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000), e)
}

/// sqrt(h + l), as `hi + lo`, of a normalized `h + l` above 0, within
/// 2^-104 of it: the `f64` root `r`, then a step of Newton's, `r + (h + l
/// - r^2) / 2r`, `r^2` exactly, and `h - r^2`'s high part exact, as they're
/// within a factor of two (Sterbenz); what the step leaves out, `(h + l -
/// r^2)^2 / 8r^3`, is below 2^-106 of it.
fn dd_sqrt(h: f64, l: f64) -> (f64, f64) with {} {
    let r = h.sqrt();
    let (p, pe) = two_prod(r, r);
    fast_two_sum(r, (h - p - pe + l) / (2.0 * r))
}

/// π/2 - (th + tl), of a `th` from 0 to π/4 and a little, so at least π/4:
/// nothing cancels.
fn pio2_less(th: f64, tl: f64) -> (f64, f64) with {} {
    let (h, e) = fast_two_sum(PIO2_H, -th);
    fast_two_sum(h, e + (PIO2_L - tl))
}

/// π/2 + (th + tl), of a `th` from 0 to π/4 and a little.
fn pio2_more(th: f64, tl: f64) -> (f64, f64) with {} {
    let (h, e) = fast_two_sum(PIO2_H, th);
    fast_two_sum(h, e + (PIO2_L + tl))
}

/// π - (th + tl), of a `th` from 0 to π/2 and a little, so at least π/2.
fn pi_less(th: f64, tl: f64) -> (f64, f64) with {} {
    let (h, e) = fast_two_sum(PI_H, -th);
    fast_two_sum(h, e + (PI_L - tl))
}

/// atan(j/64), j from 0 to 64, each as `hi + lo`, as bits
/// (scripts/math/tables.c `atan`).
const ATAN: [u64; 130] = [
    0x0000000000000000,
    0x0000000000000000,
    0x3F8FFF555BBB729B,
    0xBC2220C39D4DFF50,
    0x3F9FFD55BBA97625,
    0xBC35EC431444912C,
    0x3FA7FB818430DA2A,
    0xBC086EF8F794F105,
    0x3FAFF55BB72CFDEA,
    0xBC3C934D86D23F1D,
    0x3FB3F59F0E7C559D,
    0x3C5AC4CE285DF847,
    0x3FB7EE182602F10F,
    0xBC5CFB654C0C3D98,
    0x3FBBE39EBE6F07C3,
    0x3C5F7B8F29A05987,
    0x3FBFD5BA9AAC2F6E,
    0xBC4CD37686760C17,
    0x3FC1E1FAFB043727,
    0xBC4B485914DACF8C,
    0x3FC3D6EEE8C6626C,
    0x3C661A3B0CE9281B,
    0x3FC5C9811E3EC26A,
    0xBC5054AB2C010F3D,
    0x3FC7B97B4BCE5B02,
    0x3C5347B0B4F881CA,
    0x3FC9A6A8E96C8626,
    0x3C4CF601E7B4348E,
    0x3FCB90D7529260A2,
    0x3C217B10D2E0E5AB,
    0x3FCD77D5DF205736,
    0x3C6C648D1534597E,
    0x3FCF5B75F92C80DD,
    0x3C68AB6E3CF7AFBD,
    0x3FD09DC597D86362,
    0x3C762E47390CB865,
    0x3FD18BF5A30BF178,
    0x3C630CA4748B1BF9,
    0x3FD278372057EF46,
    0xBC7077CDD36DFC81,
    0x3FD362773707EBCC,
    0xBC6963A544B672D8,
    0x3FD44AA436C2AF0A,
    0xBC75D5E43C55B3BA,
    0x3FD530AD9951CD4A,
    0xBC62566480884082,
    0x3FD614840309CFE2,
    0xBC7A725715711F00,
    0x3FD6F61941E4DEF1,
    0xBC7C63AAE6F6E918,
    0x3FD7D5604B63B3F7,
    0x3C769C885C2B249A,
    0x3FD8B24D394A1B25,
    0x3C7B6D0BA3748FA8,
    0x3FD98CD5454D6B18,
    0x3C79E6C988FD0A77,
    0x3FDA64EEC3CC23FD,
    0xBC724DEC1B50B7FF,
    0x3FDB3A911DA65C6C,
    0x3C7AE187B1CA5040,
    0x3FDC0DB4C94EC9F0,
    0xBC7CC1CE70934C34,
    0x3FDCDE53432C1351,
    0xBC7A2CFA4418F1AD,
    0x3FDDAC670561BB4F,
    0x3C7A2B7F222F65E2,
    0x3FDE77EB7F175A34,
    0x3C70E53DC1BF3435,
    0x3FDF40DD0B541418,
    0xBC6A3992DC382A23,
    0x3FE0039C73C1A40C,
    0xBC8B32C949C9D593,
    0x3FE0657E94DB30D0,
    0xBC7D5B495F6349E6,
    0x3FE0C6145B5B43DA,
    0x3C5974FA13B5404F,
    0x3FE1255D9BFBD2A9,
    0xBC52BDAEE1C0EE35,
    0x3FE1835A88BE7C13,
    0x3C8C621CEC00C301,
    0x3FE1E00BABDEFEB4,
    0xBC5928DF287A668F,
    0x3FE23B71E2CC9E6A,
    0x3C6C421C9F38224E,
    0x3FE2958E59308E31,
    0xBC709E73B0C6C087,
    0x3FE2EE628406CBCA,
    0x3C8C5D5E9FF0CF8D,
    0x3FE345F01CCE37BB,
    0x3C81021137C71102,
    0x3FE39C391CD4171A,
    0xBC82304331D8BF46,
    0x3FE3F13FB89E96F4,
    0x3C7ECF8B492644F0,
    0x3FE445065B795B56,
    0xBC7F76D0163F79C8,
    0x3FE4978FA3269EE1,
    0x3C72419A87F2A458,
    0x3FE4E8DE5BB6EC04,
    0x3C84A33DBEB3796C,
    0x3FE538F57B89061F,
    0xBC81BB74ABDA520C,
    0x3FE587D81F732FBB,
    0xBC75E5C9D8C5A950,
    0x3FE5D58987169B18,
    0x3C60028E4BC5E7CA,
    0x3FE6220D115D7B8E,
    0xBC62B785350EE8C1,
    0x3FE66D663923E087,
    0xBC76EA6FEBE8BBBA,
    0x3FE6B798920B3D99,
    0xBC8A80386188C50E,
    0x3FE700A7C5784634,
    0xBC78C34D25AADEF6,
    0x3FE748978FBA8E0F,
    0x3C47B2A6165884A1,
    0x3FE78F6BBD5D315E,
    0x3C8406A089803740,
    0x3FE7D528289FA093,
    0x3C8560821E2F3AA9,
    0x3FE819D0B7158A4D,
    0xBC7BF76229D3B917,
    0x3FE85D69576CC2C5,
    0x3C66B66E7FC8B8C3,
    0x3FE89FF5FF57F1F8,
    0xBC855B9A5E177A1B,
    0x3FE8E17AA99CC05E,
    0xBC7EC182AB042F61,
    0x3FE921FB54442D18,
    0x3C81A62633145C07,
];

/// atan t, as `hi + lo`, of a `t = th + tl` from 0 to 1 and a little, its
/// low part at most its high one's last bit: within 2^-93 of it (lemma M1;
/// MPFR finds at most 2^-95.2). `t = c + d`, `c = j/64` the nearest, so
/// `|d| ≤ 1/128` and a little, and `atan t = atan c + atan u`, `u = (t - c)
/// / (1 + tc)`, `|u| ≤ |d|`, within 2^-99 of it: `th - c` is exact, as `c`
/// has 7 bits and is within a factor of two of `th`, or 0. Then `atan u = u
/// - u^3 Q`, `Q = 1/3 - u^2/5 + u^4/7 - .. - u^10/13`, which leaves out
/// less than 2^-101 of it; `u^3 Q` is at most 2^-15.6 of `u`: `Q`'s first
/// two terms in double-double, the rest, below 2^-30.8 of it, in `f64`, a
/// few of its last bits off, so `Q` within 2^-79 of itself and `atan u`
/// within 2^-94.6. Where `j ≥ 1`, `|atan u|` is at most half `atan c`, so
/// their sum isn't less than half of either, and cancels at most one more
/// bit.
fn atan_core(th: f64, tl: f64) -> (f64, f64) with {} {
    let jf = th * 64.0 + SHIFTER;
    let j0 = fix::clamp(wrap(jf.to_bits()), 64);
    let j = if j0 < 0 { 0 } else { j0 } as usize;
    let c = (jf - SHIFTER) * 0.015625;
    let (nh, nl) = two_sum(th - c, tl);
    let (ph, pl) = two_prod(th, c);
    let (dh0, dl0) = fast_two_sum(1.0, ph);
    let (dh, dl) = fast_two_sum(dh0, dl0 + (pl + tl * c));
    let (uh, ul) = dd_div(nh, nl, dh, dl);
    let (s2h, s2l0) = two_prod(uh, uh);
    let s2l = s2l0 + 2.0 * uh * ul;
    let mut t = -0.076923076923076927;
    t = t * s2h + 0.090909090909090912;
    t = t * s2h - 0.1111111111111111;
    t = t * s2h + 0.14285714285714285;
    t = s2h * s2h * t;
    let (p, pe) = two_prod(s2h, 0.20000000000000001);
    let (q, qe) = fast_two_sum(0.33333333333333331, -p);
    let ql = qe
        + (1.8503717077085941e-17
            - pe
            + s2h * 1.1102230246251566e-17
            - s2l * 0.20000000000000001
            + t);
    let (ch3, cl3) = dd_mul(uh, ul, s2h, s2l);
    let (vh, vl) = dd_mul(ch3, cl3, q, ql);
    let (ah, ae) = fast_two_sum(uh, -vh);
    let al = ae + (ul - vl);
    if j == 0 {
        return fast_two_sum(ah, al);
    }
    let bh = f64::from_bits(ATAN[2 * j]);
    let bl = f64::from_bits(ATAN[2 * j + 1]);
    let (sh, se) = fast_two_sum(bh, ah);
    fast_two_sum(sh, se + (bl + al))
}

/// atan a, as `hi + lo`, of an `a` above 0, within 2^-93 of it: to 1,
/// `atan_core`; then `π/2 - atan(1/a)`, `1/a` within 2^-100 of it, and
/// atan's error no more than its argument's, relatively; past 2^100, `π/2
/// - 1/a`, as what that leaves out, `1/3a^3`, is below 2^-300.
fn atan_pos(a: f64) -> (f64, f64) with {} {
    if a <= 1.0 {
        return atan_core(a, 0.0);
    }
    if a > 1.2676506002282294e30 {
        return fast_two_sum(PIO2_H, PIO2_L - 1.0 / a);
    }
    let (rh, rl) = dd_div(1.0, 0.0, a, 0.0);
    let (th, tl) = atan_core(rh, rl);
    pio2_less(th, tl)
}

/// atan x, from -π/2 to π/2: `x` itself below 2^-27, where `x^3/3` is
/// below half its last bit, and ±π/2 of ±∞.
pub fn atan_f64(x: f64) -> f64 with {} {
    if x != x {
        return x + x;
    }
    let a = abs(x);
    if a < 7.450580596923828e-9 {
        return x;
    }
    let (hi, lo) = atan_pos(a);
    match decide64(hi, lo, ARC_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f64::from_bits(fix::atan2(a, 1.0, false, x < 0.0, f64_format())),
    }
}

/// atan x of an `f32`: `x` below 2^-13.
pub fn atan_f32(x: f32) -> f32 with {} {
    if x != x {
        return x + x;
    }
    let a = abs(x as f64);
    if a < 0.0001220703125 {
        return x;
    }
    let (hi, lo) = atan_pos(a);
    match decide32(hi, lo, ARC_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f32::from_bits(wrap(fix::atan2(a, 1.0, false, x < 0.0, f32_format()))),
    }
}

/// sqrt(1 - a^2), as `hi + lo`, of an `a` from 0 to below 1, within 2^-103
/// of it: `1 - a^2` as `(1 - a)(1 + a)` from 1/2 on, where `1 - a` is exact
/// and `1 + a` is two `f64`s exactly, else as `1 - a^2`, `a^2` exactly: either
/// way within 2^-104 of it.
fn sqrt_one_less_square(a: f64) -> (f64, f64) with {} {
    let (h, l) = if a >= 0.5 {
        let (eh, el) = fast_two_sum(1.0, a);
        dd_mul(eh, el, 1.0 - a, 0.0)
    } else {
        let (p, pe) = two_prod(a, a);
        let (h, e) = fast_two_sum(1.0, -p);
        fast_two_sum(h, e - pe)
    };
    dd_sqrt(h, l)
}

/// asin a, as `hi + lo`, of an `a` from 2^-26 to below 1, within 2^-92 of
/// it: `atan(a / s)`, `s = sqrt(1 - a^2)`, where `a ≤ s`, else `π/2 - atan(s
/// / a)`, which is at least π/4; the quotient within 2^-100 of it, and
/// `s`'s error, 2^-103, besides.
fn asin_pos(a: f64) -> (f64, f64) with {} {
    let (sh, sl) = sqrt_one_less_square(a);
    if a <= sh {
        let (qh, ql) = dd_div(a, 0.0, sh, sl);
        atan_core(qh, ql)
    } else {
        let (qh, ql) = dd_div(sh, sl, a, 0.0);
        let (th, tl) = atan_core(qh, ql);
        pio2_less(th, tl)
    }
}

/// asin x, from -π/2 to π/2: a NaN past ±1, and `x` itself below 2^-26,
/// where `x^3/6` is below half its last bit.
pub fn asin_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 1.0) {
        return if a == 1.0 {
            if x < 0.0 { -PIO2_H } else { PIO2_H }
        } else {
            f64::nan() + x
        };
    }
    if a < 1.4901161193847656e-8 {
        return x;
    }
    let (hi, lo) = asin_pos(a);
    match decide64(hi, lo, ARC_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f64::from_bits(fix::asin(a, x < 0.0, f64_format())),
    }
}

/// asin x of an `f32`: `x` below 2^-12.
pub fn asin_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < 1.0) {
        return if a == 1.0 {
            if x < 0.0 { -1.5707964 } else { 1.5707964 }
        } else {
            f32::nan() + x
        };
    }
    if a < 0.000244140625 {
        return x;
    }
    let (hi, lo) = asin_pos(a);
    match decide32(hi, lo, ARC_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f32::from_bits(wrap(fix::asin(a, x < 0.0, f32_format()))),
    }
}

/// acos x, as `hi + lo`, of an `x` of magnitude `a` from 2^-60 to below 1,
/// within 2^-92 of it: of `asin`'s parts, `π/2 ∓ atan(a / s)` where `a ≤ s`,
/// at least π/4, else `atan(s / a)` of an `x` above 0, and `π - atan(s / a)`,
/// at least 3π/4, below.
fn acos_core(x: f64) -> (f64, f64) with {} {
    let a = abs(x);
    let (sh, sl) = sqrt_one_less_square(a);
    if a <= sh {
        let (qh, ql) = dd_div(a, 0.0, sh, sl);
        let (th, tl) = atan_core(qh, ql);
        if x < 0.0 { pio2_more(th, tl) } else { pio2_less(th, tl) }
    } else {
        let (qh, ql) = dd_div(sh, sl, a, 0.0);
        let (th, tl) = atan_core(qh, ql);
        if x < 0.0 { pi_less(th, tl) } else { (th, tl) }
    }
}

/// acos x, from 0 to π: a NaN past ±1, and π/2 below 2^-60, where `π/2 -
/// x` rounds to the `f64` nearest π/2.
pub fn acos_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 1.0) {
        return if x == 1.0 { 0.0 } else if x == -1.0 { PI_H } else { f64::nan() + x };
    }
    if a < 8.673617379884035e-19 {
        return PIO2_H;
    }
    let (hi, lo) = acos_core(x);
    match decide64(hi, lo, ARC_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::acos(x, f64_format())),
    }
}

/// acos x of an `f32`: π/2 below 2^-30.
pub fn acos_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    let a = abs(d);
    if !(a < 1.0) {
        return if x == 1.0 { 0.0 } else if x == -1.0 { 3.1415927 } else { f32::nan() + x };
    }
    if a < 9.313225746154785e-10 {
        return 1.5707964;
    }
    let (hi, lo) = acos_core(d);
    match decide32(hi, lo, ARC_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::acos(d, f32_format()))),
    }
}

/// atan(a / b), or `π - atan(a / b)` if `left`, the angle of the point `(±b,
/// a)`, of finite `a` and `b` above 0, as `(hi + lo) · 2^m`, within 2^-92 of
/// it. With `a = ma · 2^ea` and `b = mb · 2^eb`, `ma` and `mb` from 1 to 2:
/// where `ea - eb` is over 60, `a / b` is above 2^59, and the angle `π/2 ∓ b
/// / a`, as `atan(a / b) = π/2 - atan(b / a)` and `b / a` is below 2^-59;
/// where it's below -60, `π - a / b` if `left`, else `atan(a / b)` is `a /
/// b` but for less than 2^-118 of it, as `(ma / mb) · 2^(ea - eb)`, scaled
/// as `m` says, as it may be subnormal; else `ma · 2^(ea - eb)`, exactly,
/// and `mb` are as `a` and `b` but both normal and of moderate size, so
/// their quotient, or the other's, is `atan_core`'s.
fn atan2_core(a: f64, b: f64, left: bool) -> (f64, f64, i32) with {} {
    let (ma, ea) = parts(a);
    let (mb, eb) = parts(b);
    let d = ea - eb;
    if d > 60 {
        let r = b / a;
        let (h, l) = if left {
            fast_two_sum(PIO2_H, PIO2_L + r)
        } else {
            fast_two_sum(PIO2_H, PIO2_L - r)
        };
        return (h, l, 0);
    }
    if d < -60 {
        if left {
            let (h, l) = fast_two_sum(PI_H, PI_L - a / b);
            return (h, l, 0);
        }
        let (qh, ql) = dd_div(ma, 0.0, mb, 0.0);
        return (qh, ql, d);
    }
    let a2 = ma * two_to(d);
    let (th, tl) = if a2 <= mb {
        let (qh, ql) = dd_div(a2, 0.0, mb, 0.0);
        atan_core(qh, ql)
    } else {
        let (qh, ql) = dd_div(mb, 0.0, a2, 0.0);
        let (th, tl) = atan_core(qh, ql);
        pio2_less(th, tl)
    };
    let (h, l) = if left { pi_less(th, tl) } else { (th, tl) };
    (h, l, 0)
}

/// atan2(y, x), from -π to π: the angle of the point `(x, y)`, its sign
/// `y`'s, of `-0.0` too. IEEE 754-2019's values where `x` or `y` is 0 or an
/// infinity (§9.2.1), which are C's: of `y = ±0`, ±0 where `x` is positive
/// or +0, else ±π; of `x = ±0`, ±π/2; of `x = ±∞`, ±0 or ±π, but where `y`
/// is an infinity too, which makes them ±π/4 and ±3π/4; and of `y = ±∞`,
/// else, ±π/2.
pub fn atan2_f64(y: f64, x: f64) -> f64 with {} {
    if x != x || y != y {
        return x + y;
    }
    let a = abs(y);
    let b = abs(x);
    let left = x.is_sign_negative();
    let infinity = f64::infinity();
    let r = if a == 0.0 {
        if left { PI_H } else { 0.0 }
    } else if a == infinity {
        if b == infinity {
            if left { 2.3561944901923448 } else { PIO4_H }
        } else {
            PIO2_H
        }
    } else if b == 0.0 {
        PIO2_H
    } else if b == infinity {
        if left { PI_H } else { 0.0 }
    } else {
        let (hi, lo, m) = atan2_core(a, b, left);
        let fast = if m > -1022 {
            match decide64(hi, lo, ARC_EPS) {
                Option::Some(r) => Option::Some(scale(r, m)),
                Option::None => Option::None,
            }
        } else {
            Option::None
        };
        match fast {
            Option::Some(r) => r,
            Option::None => f64::from_bits(fix::atan2(a, b, left, false, f64_format())),
        }
    };
    if y.is_sign_negative() { -r } else { r }
}

/// atan2(y, x) of `f32`s, as of `f64`s: their quotient is never past
/// 2^±277, so the angle never subnormal as an `f64`.
pub fn atan2_f32(y: f32, x: f32) -> f32 with {} {
    if x != x || y != y {
        return x + y;
    }
    let a = abs(y as f64);
    let b = abs(x as f64);
    let left = x.is_sign_negative();
    let infinity = f64::infinity();
    let r: f32 = if a == 0.0 {
        if left { 3.1415927 } else { 0.0 }
    } else if a == infinity {
        if b == infinity {
            if left { 2.3561945 } else { 0.78539819 }
        } else {
            1.5707964
        }
    } else if b == 0.0 {
        1.5707964
    } else if b == infinity {
        if left { 3.1415927 } else { 0.0 }
    } else {
        let (hi, lo, m) = atan2_core(a, b, left);
        match decide32(scale(hi, m), scale(lo, m), ARC_EPS) {
            Option::Some(r) => r,
            Option::None => f32::from_bits(wrap(fix::atan2(a, b, left, false, f32_format()))),
        }
    };
    if y.is_sign_negative() { -r } else { r }
}

/// cbrt u, as `hi + lo`, of a `u` from 1 to below 8, within 2^-100 of it:
/// Newton's iteration for `y^3 = u`, `y - (y^3 - u) / 3y^2`, which squares
/// its error, five times in `f64` from a first guess within 10% (a third of
/// the bits of `u` above 1's), to 2^-52; then once more in double-double,
/// `y^3` within 2^-102 of it, and `u - y^3`'s high part exact, as they're
/// within a factor of two (Sterbenz); what that leaves out, `(u - y^3)^2 /
/// 9y^5`, is below 2^-104 of it.
fn cbrt_core(u: f64) -> (f64, f64) with {} {
    let mut y = f64::from_bits(
        wrapping_sub(u.to_bits(), 0x3FF0_0000_0000_0000) / 3 + 0x3FF0_0000_0000_0000,
    );
    for _ in 0..5u32 {
        y = y - (y * y * y - u) / (3.0 * y * y);
    }
    let (p, pe) = two_prod(y, y);
    let (c, ce) = dd_mul(p, pe, y, 0.0);
    fast_two_sum(y, (u - c - ce) / (3.0 * p))
}

/// For `cbrt` of `|x| = m · 2^e`, `m` from 1 to 2: `u = m · 2^r`, from 1 to
/// below 8, and `k`, of `e = 3k + r`, `r` from 0 to 2, so `cbrt |x| = cbrt u
/// · 2^k`.
fn cbrt_reduce(a: f64) -> (f64, i32) with {} {
    let (m, e) = parts(a);
    let k = (e + 3075) / 3 - 1025;
    let r = e - 3 * k;
    (if r == 0 { m } else if r == 1 { 2.0 * m } else { 4.0 * m }, k)
}

/// The cube root, of a negative `x` too: exact where it's a float, and `x`
/// itself of 0 and of the infinities.
pub fn cbrt_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < f64::infinity()) || a == 0.0 {
        return x + x;
    }
    let (u, k) = cbrt_reduce(a);
    let (hi, lo) = cbrt_core(u);
    let y = match decide64(hi, lo, ARC_EPS) {
        Option::Some(y) => y,
        Option::None => {
            // The ends `decide64` rounded, next to each other: the root is
            // on one side of their midpoint, as the cube of a number of 54
            // bits has more than 53, and which, `u` says, exactly.
            let err = abs(hi) * ARC_EPS;
            let l = hi + (lo - err);
            let h = hi + (lo + err);
            if fix::cube_above(u, l, h) { h } else { l }
        },
    };
    let r = scale(y, k);
    if x < 0.0 { -r } else { r }
}

/// The cube root of an `f32`.
pub fn cbrt_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < f64::infinity()) || a == 0.0 {
        return x + x;
    }
    let (u, k) = cbrt_reduce(a);
    let (hi, lo) = cbrt_core(u);
    let y = match decide32(hi, lo, ARC_EPS) {
        Option::Some(y) => y,
        Option::None => {
            let (l, h) = ends32(hi, lo, ARC_EPS);
            if fix::cube_above(u, l as f64, h as f64) { h } else { l }
        },
    };
    let r = f32::nearest_f64(scale(y as f64, k));
    if x < 0.0 { -r } else { r }
}

/// sqrt(x^2 + y^2), as `hi + lo`, of a `y` at most `x`, within 2^-103 of it:
/// each square exactly, as two `f64`s, and their sum within 2^-105.
fn hypot_core(x: f64, y: f64) -> (f64, f64) with {} {
    let (p1, e1) = two_prod(x, x);
    let (p2, e2) = two_prod(y, y);
    let (s, se) = fast_two_sum(p1, p2);
    let (h, l) = fast_two_sum(s, se + (e1 + e2));
    dd_sqrt(h, l)
}

/// Of `hypot`'s two candidates `l` and `h`, next to each other, the one
/// nearer `sqrt(x^2 + y^2)`, by the sign of `x^2 + y^2` less their
/// midpoint's square, worked out exactly (`fix::square_side`); where it's
/// 0, the one of them that's even.
fn hypot_pick(x: f64, y: f64, l: f64, h: f64) -> f64 with {} {
    let side = fix::square_side(x, y, l, h);
    if side > 0 { h } else if side < 0 { l } else if l.to_bits() % 2 == 0 { l } else { h }
}

/// hypot of subnormal `a ≥ b`, as integers `A = a · 2^1074` and `B`, below
/// 2^52: the float nearest `sqrt(A^2 + B^2) · 2^-1074` is the integer
/// nearest the root, `N`, below 2^53, times 2^-1074, exactly, normal or not;
/// never halfway, as `(N + 1/2)^2` isn't an integer. `x = a · 2^1022` and `y`
/// are `A` and `B` over 2^52, less than 1, and the root, worked out to
/// 2^-103 of it, gives `N` but where it's within that of halfway, which
/// the exact comparison of both sides then settles.
fn hypot_subnormal(a: f64, b: f64) -> f64 with {} {
    let x = a * two_to(1022);
    let y = b * two_to(1022);
    let (hi, lo) = hypot_core(x, y);
    let n0 = (hi * 4503599627370496.0 + lo * 4503599627370496.0).round();
    let u = 2.220446049250313e-16;
    let n = if fix::square_side(x, y, (n0 - 1.0) * u, n0 * u) < 0 {
        n0 - 1.0
    } else if fix::square_side(x, y, n0 * u, (n0 + 1.0) * u) > 0 {
        n0 + 1.0
    } else {
        n0
    };
    n * 4.9406564584124654e-324
}

/// sqrt(x^2 + y^2), without overflowing or underflowing on the way, and
/// exact where it's a float: +∞ if either is infinite, even where the
/// other is a NaN (IEEE 754-2019, §9.2.1), and `|x|` where `|y|` is below
/// 2^-27 of it, as `y^2/2x` is then below half its last bit. Else, of the
/// greater `a = ma · 2^ea`, `ma` from 1 to 2, and the lesser `b`, scaled
/// alike: `2^ea · hypot(ma, b · 2^-ea)`, each of them exactly; and where the
/// fast path can't tell, the exact sum of the squares can, as of
/// `hypot_pick`: halfway between two floats is possible, as `5` is of `3`
/// and `4`.
pub fn hypot_f64(x: f64, y: f64) -> f64 with {} {
    let ax = abs(x);
    let ay = abs(y);
    let infinity = f64::infinity();
    if ax == infinity || ay == infinity {
        return infinity;
    }
    if x != x || y != y {
        return x + y;
    }
    let (a, b) = if ax < ay { (ay, ax) } else { (ax, ay) };
    if b == 0.0 {
        return a;
    }
    if a < 2.2250738585072014e-308 {
        return hypot_subnormal(a, b);
    }
    let (ma, ea) = parts(a);
    let (mb0, eb) = parts(b);
    let d = if ea - eb < 0 { 0 } else { ea - eb };
    if d > 27 {
        return a;
    }
    let mb = mb0 * two_to(-d);
    let (hi, lo) = hypot_core(ma, mb);
    let r = match decide64(hi, lo, ARC_EPS) {
        Option::Some(r) => r,
        Option::None => {
            let err = abs(hi) * ARC_EPS;
            hypot_pick(ma, mb, hi + (lo - err), hi + (lo + err))
        },
    };
    // `scale` takes an exponent from -1021 up: of the least, half `r`.
    if r >= 2.0 || ea < -1021 { scale(0.5 * r, ea + 1) } else { scale(r, ea) }
}

/// hypot of `f32`s: in `f64`s, where the squares and their sum are
/// exact, as two `f64`s, and so is the square of the midpoint of two
/// `f32`s, so the comparison where the fast path can't tell is too.
pub fn hypot_f32(x: f32, y: f32) -> f32 with {} {
    let ax = abs(x as f64);
    let ay = abs(y as f64);
    let infinity = f64::infinity();
    if ax == infinity || ay == infinity {
        return f32::infinity();
    }
    if x != x || y != y {
        return x + y;
    }
    let (p1, _) = two_prod(ax, ax);
    let (p2, _) = two_prod(ay, ay);
    let (s, se) = two_sum(p1, p2);
    let (hi, lo) = dd_sqrt(s, se);
    match decide32(hi, lo, ARC_EPS) {
        Option::Some(r) => r,
        Option::None => {
            let (l, h) = ends32(hi, lo, ARC_EPS);
            let m = (l as f64 + h as f64) * 0.5;
            let d = m * m - s - se;
            if d < 0.0 { h } else if d > 0.0 { l } else if l.to_bits() % 2 == 0 { l } else { h }
        },
    }
}

/// ln(1 + u), as `hi + lo`, of a `u = uh + ul` above 0, normalized: as
/// `ln1p_core`, within `LN_EPS · |hi|` of it.
fn ln1p_dd(uh: f64, ul: f64) -> (f64, f64) with {} {
    if uh < 3.0517578125e-5 {
        return ln1p_small(uh, ul);
    }
    let (s, e) = two_sum(1.0, uh);
    let (h, l) = ln_core(s);
    fast_two_sum(h, l + (e + ul) / s)
}

/// asinh a, as `hi + lo`, of an `a` from 2^-26 up, within `LN_EPS · |hi|`:
/// to 2^28, `ln(1 + u)`, `u = a + a^2 / (1 + sqrt(1 + a^2))`, which is `a +
/// sqrt(1 + a^2) - 1` without its cancelling, its terms all positive, so
/// within 2^-100 of it, which `ln(1 + u)` is no more sensitive to; past
/// 2^28, `ln a + ln 2 + 1/4a^2`, which leaves out less than 2^-114 of it.
fn asinh_core(a: f64) -> (f64, f64) with {} {
    if a > 268435456.0 {
        let (h, l) = ln_core(a);
        let (s, se) = fast_two_sum(h, LN2_H);
        return fast_two_sum(s, se + (l + LN2_L + 0.25 / a / a));
    }
    let (p, pe) = two_prod(a, a);
    let (q0, qe) = two_sum(1.0, p);
    let (qh, ql) = fast_two_sum(q0, qe + pe);
    let (rh, rl) = dd_sqrt(qh, ql);
    let (d0, de) = two_sum(1.0, rh);
    let (dh, dl) = fast_two_sum(d0, de + rl);
    let (wh, wl) = dd_div(p, pe, dh, dl);
    let (u0, ue) = two_sum(a, wh);
    let (uh, ul) = fast_two_sum(u0, ue + wl);
    ln1p_dd(uh, ul)
}

/// asinh x: `x` itself below 2^-26, where `x^3/6` is below half its last
/// bit, and ±∞ of ±∞.
pub fn asinh_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < f64::infinity()) {
        return x + x;
    }
    if a < 1.4901161193847656e-8 {
        return x;
    }
    let (hi, lo) = asinh_core(a);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f64::from_bits(fix::asinh(a, x < 0.0, f64_format())),
    }
}

/// asinh x of an `f32`: `x` below 2^-12.
pub fn asinh_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < f64::infinity()) {
        return x + x;
    }
    if a < 0.000244140625 {
        return x;
    }
    let (hi, lo) = asinh_core(a);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f32::from_bits(wrap(fix::asinh(a, x < 0.0, f32_format()))),
    }
}

/// acosh x, as `hi + lo`, of an `x` above 1, within `LN_EPS · |hi|`: below 2,
/// `ln(1 + u)`, `u = t + sqrt(t (2 + t))`, of `t = x - 1`, exact, its terms
/// positive; to 2^28, `ln(x + sqrt(x^2 - 1))`, `x^2 - 1` at least 3, so
/// nothing cancels; past 2^28, `ln x + ln 2 - 1/4x^2`, which leaves out less
/// than 2^-114 of it.
fn acosh_core(x: f64) -> (f64, f64) with {} {
    if x > 268435456.0 {
        let (h, l) = ln_core(x);
        let (s, se) = fast_two_sum(h, LN2_H);
        return fast_two_sum(s, se + (l + LN2_L - 0.25 / x / x));
    }
    if x < 2.0 {
        let t = x - 1.0;
        let (gh, gl) = fast_two_sum(2.0, t);
        let (ph, pl) = dd_mul(gh, gl, t, 0.0);
        let (rh, rl) = dd_sqrt(ph, pl);
        let (u0, ue) = fast_two_sum(rh, t);
        let (uh, ul) = fast_two_sum(u0, ue + rl);
        return ln1p_dd(uh, ul);
    }
    let (p, pe) = two_prod(x, x);
    let (q0, qe) = fast_two_sum(p, -1.0);
    let (qh, ql) = fast_two_sum(q0, qe + pe);
    let (rh, rl) = dd_sqrt(qh, ql);
    let (v0, ve) = fast_two_sum(x, rh);
    let (vh, vl) = fast_two_sum(v0, ve + rl);
    let (h, l) = ln_core(vh);
    fast_two_sum(h, l + vl / vh)
}

/// acosh x: 0 of 1, a NaN below 1, and ∞ of ∞.
pub fn acosh_f64(x: f64) -> f64 with {} {
    if !(x > 1.0) || x == f64::infinity() {
        return if x == 1.0 { 0.0 } else if x > 1.0 { x } else { f64::nan() + x };
    }
    let (hi, lo) = acosh_core(x);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::acosh(x, f64_format())),
    }
}

/// acosh x of an `f32`.
pub fn acosh_f32(x: f32) -> f32 with {} {
    if !(x > 1.0) || x == f32::infinity() {
        return if x == 1.0 { 0.0 } else if x > 1.0 { x } else { f32::nan() + x };
    }
    let d = x as f64;
    let (hi, lo) = acosh_core(d);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::acosh(d, f32_format()))),
    }
}

/// atanh a, as `hi + lo`, of an `a` from 2^-27 to below 1, within `LN_EPS ·
/// |hi|`: `(ln(1 + a) - ln(1 - a)) / 2`, of terms of opposite signs, so a
/// sum that cancels nothing, each within `LN_EPS` of it (`ln1p_core`).
fn atanh_core(a: f64) -> (f64, f64) with {} {
    let (ph, pl) = ln1p_core(a);
    let (mh, ml) = ln1p_core(-a);
    let (s, se) = two_sum(ph, -mh);
    let (h, l) = fast_two_sum(s, se + (pl - ml));
    (0.5 * h, 0.5 * l)
}

/// atanh x: ±∞ of ±1, a NaN past them, and `x` itself below 2^-27, where
/// `x^3/3` is below half its last bit.
pub fn atanh_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 1.0) {
        return if a == 1.0 { x * f64::infinity() } else { f64::nan() + x };
    }
    if a < 7.450580596923828e-9 {
        return x;
    }
    let (hi, lo) = atanh_core(a);
    match decide64(hi, lo, LN_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f64::from_bits(fix::atanh(a, x < 0.0, f64_format())),
    }
}

/// atanh x of an `f32`: `x` below 2^-12.
pub fn atanh_f32(x: f32) -> f32 with {} {
    let a = abs(x as f64);
    if !(a < 1.0) {
        return if a == 1.0 { x * f32::infinity() } else { f32::nan() + x };
    }
    if a < 0.000244140625 {
        return x;
    }
    let (hi, lo) = atanh_core(a);
    match decide32(hi, lo, LN_EPS) {
        Option::Some(y) => if x < 0.0 { -y } else { y },
        Option::None => f32::from_bits(wrap(fix::atanh(a, x < 0.0, f32_format()))),
    }
}

/// For `sin_pi` and `cos_pi`, of an `a` from 0 to below 2^52: `a = n + k/2 +
/// r`, `n` an integer, `k` from 0 to 2 and `r` from -1/4 to 1/4, each exactly:
/// `n` is `a`'s integer part, and `f = a - n`, from 0 to below 1, is exact,
/// and so is `r = f - k/2`, as `k/2` is within a factor of two of `f`, or 0
/// (Sterbenz); and the quarter turn `πa` is in, mod 4, `2n + k`, which `n`'s
/// last bit says, as the last of `n + 2^52`, exact.
fn pi_reduce(a: f64) -> (f64, { q: u32 | q < 4 }) with {} {
    let n = a.trunc();
    let f = a - n;
    let odd = (n + 4503599627370496.0).to_bits() % 2 == 1;
    let (r, k) = if f <= 0.25 { (f, 0) } else if f < 0.75 { (f - 0.5, 1) } else { (f - 1.0, 2) };
    (r, (if odd { 2 } else { 0 } + k) % 4)
}

/// `sin πr` and `cos πr`, each as `hi + lo`, of a nonzero `r` from -1/4
/// to 1/4 and of magnitude at least 2^-200: `πr` in double-double, within
/// 2^-104 of it, then `sincos_core`, within 2^-90 of each, and below 1/128,
/// where its table isn't used, within that of each relatively: so at most
/// 2^-82 of each (MPFR finds 2^-82.3), well within `TRIG_EPS`.
fn sincos_pi(r: f64) -> (f64, f64, f64, f64) with {} {
    let (ph, pl) = two_prod(PI_H, r);
    sincos_core(ph, pl + PI_L * r)
}

/// sin πx, worked out from `x` exactly, so precise where `x` is big, where
/// `(π * x).sin()` isn't: 0 of an integer, with `x`'s sign (IEEE 754-2019,
/// §9.2.1), ±1 halfway between two, and a NaN of an infinity. Below 2^-52
/// it's `πx`, but for less than 2^-102 of it: of `x = m · 2^e`, `m` from 1 to
/// 2, it's `(π/4 · m) · 2^(e + 2)`, each in double-double.
pub fn sin_pi_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 4503599627370496.0) {
        return if x != x || a == f64::infinity() { x - x } else { 0.0 * x };
    }
    if a < 2.220446049250313e-16 {
        if a == 0.0 {
            return x;
        }
        let (m, e) = parts(a);
        let (ph, pl) = two_prod(PIO4_H, m);
        if e > -1023 {
            match decide64(ph, pl + PIO4_L * m, ARC_EPS) {
                Option::Some(y) => return if x < 0.0 { -scale(y, e + 2) } else { scale(y, e + 2) },
                Option::None => {},
            }
        }
        return f64::from_bits(fix::sin_pi_tiny(a, x < 0.0, f64_format()));
    }
    let (r, q) = pi_reduce(a);
    let y = if r == 0.0 {
        if q == 1 { 1.0 } else if q == 3 { -1.0 } else { 0.0 }
    } else {
        let (sh, sl, ch, cl) = sincos_pi(r);
        let (hi, lo) = match q {
            0 => (sh, sl),
            1 => (ch, cl),
            2 => (-sh, -sl),
            _ => (-ch, -cl),
        };
        match decide64(hi, lo, TRIG_EPS) {
            Option::Some(y) => y,
            Option::None => f64::from_bits(fix::sin_cos_pi(r, q, false, f64_format())),
        }
    };
    if x < 0.0 { -y } else { y }
}

/// cos πx: 1 of an even integer, -1 of an odd one, and +0 halfway between
/// two (IEEE 754-2019, §9.2.1); 1 below 2^-29, where `(πx)^2/2` is below half
/// the last bit below 1.
pub fn cos_pi_f64(x: f64) -> f64 with {} {
    let a = abs(x);
    if !(a < 4503599627370496.0) {
        if x != x || a == f64::infinity() {
            return x - x;
        }
        // An integer, of 53 bits or more, so even, or its last bit says.
        return if a < 9007199254740992.0 && a.to_bits() % 2 == 1 { -1.0 } else { 1.0 };
    }
    if a < 1.862645149230957e-9 {
        return 1.0;
    }
    let (r, q) = pi_reduce(a);
    if r == 0.0 {
        return if q == 0 { 1.0 } else if q == 2 { -1.0 } else { 0.0 };
    }
    let (sh, sl, ch, cl) = sincos_pi(r);
    let (hi, lo) = match q {
        0 => (ch, cl),
        1 => (-sh, -sl),
        2 => (-ch, -cl),
        _ => (sh, sl),
    };
    match decide64(hi, lo, TRIG_EPS) {
        Option::Some(y) => y,
        Option::None => f64::from_bits(fix::sin_cos_pi(r, q, true, f64_format())),
    }
}

/// sin πx of an `f32`, of which none is below 2^-149, so `πr` is never
/// subnormal as an `f64`.
pub fn sin_pi_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    let a = abs(d);
    if !(a < 4503599627370496.0) {
        return if x != x || a == f64::infinity() { x - x } else { 0.0 * x };
    }
    if a == 0.0 {
        return x;
    }
    let (r, q) = pi_reduce(a);
    let y: f32 = if r == 0.0 {
        if q == 1 { 1.0 } else if q == 3 { -1.0 } else { 0.0 }
    } else {
        let (sh, sl, ch, cl) = sincos_pi(r);
        let (hi, lo) = match q {
            0 => (sh, sl),
            1 => (ch, cl),
            2 => (-sh, -sl),
            _ => (-ch, -cl),
        };
        match decide32(hi, lo, TRIG_EPS) {
            Option::Some(y) => y,
            Option::None => f32::from_bits(wrap(fix::sin_cos_pi(r, q, false, f32_format()))),
        }
    };
    if x < 0.0 { -y } else { y }
}

/// cos πx of an `f32`: 1 below 2^-14.
pub fn cos_pi_f32(x: f32) -> f32 with {} {
    let d = x as f64;
    let a = abs(d);
    if !(a < 4503599627370496.0) {
        if x != x || a == f64::infinity() {
            return x - x;
        }
        return 1.0;
    }
    if a < 0.00006103515625 {
        return 1.0;
    }
    let (r, q) = pi_reduce(a);
    if r == 0.0 {
        return if q == 0 { 1.0 } else if q == 2 { -1.0 } else { 0.0 };
    }
    let (sh, sl, ch, cl) = sincos_pi(r);
    let (hi, lo) = match q {
        0 => (ch, cl),
        1 => (-sh, -sl),
        2 => (-ch, -cl),
        _ => (sh, sl),
    };
    match decide32(hi, lo, TRIG_EPS) {
        Option::Some(y) => y,
        Option::None => f32::from_bits(wrap(fix::sin_cos_pi(r, q, true, f32_format()))),
    }
}
