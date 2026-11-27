/* The correctly rounded value of each of core's math functions, by MPFR:
 * the oracle T's are checked against (docs/102-math.md). MPFR rounds the
 * exact value once, to nearest, ties to even, at the precision and
 * exponent range of binary32 or binary64, subnormals included, and its
 * special values are C's Annex F, which are IEEE 754-2019's.
 *
 * Header only: `oracle.c` reads queries from its input, `check.c` links
 * T's functions and compares. */
#include <mpfr.h>
#include <stdint.h>
#include <string.h>

enum { F32, F64 };

typedef int (*unary)(mpfr_ptr, mpfr_srcptr, mpfr_rnd_t);
typedef int (*binary)(mpfr_ptr, mpfr_srcptr, mpfr_srcptr, mpfr_rnd_t);

/* `pow` as T names it, MPFR's `mpfr_pow`; `ln` is `mpfr_log`. */
static const struct {
    const char *name;
    unary f1;
    binary f2;
} FUNCTIONS[] = {
    {"exp", mpfr_exp, 0},     {"exp2", mpfr_exp2, 0},   {"exp_m1", mpfr_expm1, 0},
    {"ln", mpfr_log, 0},      {"log2", mpfr_log2, 0},   {"log10", mpfr_log10, 0},
    {"ln_1p", mpfr_log1p, 0}, {"sin", mpfr_sin, 0},     {"cos", mpfr_cos, 0},
    {"tan", mpfr_tan, 0},     {"asin", mpfr_asin, 0},   {"acos", mpfr_acos, 0},
    {"atan", mpfr_atan, 0},   {"sinh", mpfr_sinh, 0},   {"cosh", mpfr_cosh, 0},
    {"tanh", mpfr_tanh, 0},   {"asinh", mpfr_asinh, 0}, {"acosh", mpfr_acosh, 0},
    {"atanh", mpfr_atanh, 0}, {"cbrt", mpfr_cbrt, 0},   {"sqrt", mpfr_sqrt, 0},
    {"pow", 0, mpfr_pow},     {"atan2", 0, mpfr_atan2}, {"hypot", 0, mpfr_hypot},
};
enum { N_FUNCTIONS = sizeof FUNCTIONS / sizeof FUNCTIONS[0] };

static int function_named(const char *name) {
    for (int i = 0; i < N_FUNCTIONS; i++)
        if (strcmp(FUNCTIONS[i].name, name) == 0) return i;
    return -1;
}

static int arity(int f) { return FUNCTIONS[f].f1 ? 1 : 2; }

static void format_range(int type) {
    /* MPFR's exponents are of a significand in [1/2, 1): binary64's
     * normal ones are 2^-1022 to below 2^1024, so -1021 to 1024, and its
     * least subnormal 2^-1074 has exponent -1073. */
    if (type == F64) {
        mpfr_set_emin(-1073);
        mpfr_set_emax(1024);
    } else {
        mpfr_set_emin(-148);
        mpfr_set_emax(128);
    }
}

static double of_bits64(uint64_t b) {
    double d;
    memcpy(&d, &b, 8);
    return d;
}

static uint64_t bits64(double d) {
    uint64_t b;
    memcpy(&b, &d, 8);
    return b;
}

static float of_bits32(uint32_t b) {
    float f;
    memcpy(&f, &b, 4);
    return f;
}

static uint32_t bits32(float f) {
    uint32_t b;
    memcpy(&b, &f, 4);
    return b;
}

/* The correctly rounded `f(x)` or `f(x, y)`, as bits of `type`; a NaN is
 * always the quiet one with no sign, as which NaN isn't said. Not
 * reentrant across threads with different `type`s: the exponent range is
 * MPFR's per thread, which is how it's built here (thread-local). */
static uint64_t oracle(int f, int type, uint64_t xb, uint64_t yb) {
    int prec = type == F64 ? 53 : 24;
    format_range(type);
    mpfr_t x, y, r;
    mpfr_init2(x, prec);
    mpfr_init2(y, prec);
    mpfr_init2(r, prec);
    if (type == F64) {
        mpfr_set_d(x, of_bits64(xb), MPFR_RNDN);
        mpfr_set_d(y, of_bits64(yb), MPFR_RNDN);
    } else {
        mpfr_set_flt(x, of_bits32((uint32_t)xb), MPFR_RNDN);
        mpfr_set_flt(y, of_bits32((uint32_t)yb), MPFR_RNDN);
    }
    int t = FUNCTIONS[f].f1 ? FUNCTIONS[f].f1(r, x, MPFR_RNDN) : FUNCTIONS[f].f2(r, x, y, MPFR_RNDN);
    t = mpfr_check_range(r, t, MPFR_RNDN);
    mpfr_subnormalize(r, t, MPFR_RNDN);
    uint64_t out;
    if (mpfr_nan_p(r))
        out = type == F64 ? 0x7FF8000000000000u : 0x7FC00000u;
    else if (type == F64)
        out = bits64(mpfr_get_d(r, MPFR_RNDN));
    else
        out = bits32(mpfr_get_flt(r, MPFR_RNDN));
    mpfr_clears(x, y, r, (mpfr_ptr)0);
    return out;
}

/* How hard `f(x)` is to round to nearest: how many bits after the
 * rounding bit are all the opposite of it, of the exact value worked out
 * to 192 bits (0 to about 130). An exact value that close to halfway
 * between two floats needs that many more bits than the format has to
 * round. A value close to a float is easy: it rounds to that float from
 * either side. 300 of an exact halfway value, -1 of infinities, NaNs and
 * zeros. Subnormal results round at fewer bits than the format has,
 * which this doesn't model: they're rare, and have tests of their own. */
static int hardness(int f, int type, uint64_t xb, uint64_t yb) {
    int prec = type == F64 ? 53 : 24;
    mpfr_set_emin(mpfr_get_emin_min());
    mpfr_set_emax(mpfr_get_emax_max());
    mpfr_t x, y, r, d;
    mpfr_init2(x, prec);
    mpfr_init2(y, prec);
    mpfr_init2(r, 192);
    mpfr_init2(d, 192);
    if (type == F64) {
        mpfr_set_d(x, of_bits64(xb), MPFR_RNDN);
        mpfr_set_d(y, of_bits64(yb), MPFR_RNDN);
    } else {
        mpfr_set_flt(x, of_bits32((uint32_t)xb), MPFR_RNDN);
        mpfr_set_flt(y, of_bits32((uint32_t)yb), MPFR_RNDN);
    }
    if (FUNCTIONS[f].f1)
        FUNCTIONS[f].f1(r, x, MPFR_RNDN);
    else
        FUNCTIONS[f].f2(r, x, y, MPFR_RNDN);
    int h = -1;
    if (mpfr_regular_p(r)) {
        /* The significand scaled to [2^(p-1), 2^p): its fraction is the
         * bits past the format's, the rounding bit first, and `d` how far
         * that is from a half. */
        mpfr_abs(d, r, MPFR_RNDN);
        mpfr_mul_2si(d, d, prec - mpfr_get_exp(r), MPFR_RNDN);
        mpfr_frac(d, d, MPFR_RNDN);
        mpfr_sub_d(d, d, 0.5, MPFR_RNDN);
        mpfr_abs(d, d, MPFR_RNDN);
        h = mpfr_zero_p(d) ? 300 : (int)-mpfr_get_exp(d) - 1;
    }
    mpfr_clears(x, y, r, d, (mpfr_ptr)0);
    return h;
}
