/* Checks core's math functions, compiled from T, against MPFR: every
 * input of an `f32`, or random ones of an `f64` (docs/102-math.md).
 *
 *     scripts/math/check.sh exp f32 all
 *     scripts/math/check.sh exp f64 bits 1000000 [seed]
 *     scripts/math/check.sh exp f64 range -745 710 1000000 [seed]
 *     scripts/math/check.sh pow f64 range2 0 4 -60 60 1000000 [seed]
 *     scripts/math/check.sh exp f64 list < inputs   (hex bits, a line each)
 *
 * `bits` draws uniform bit patterns, so every exponent equally; `range`
 * values uniform in [lo, hi]; `range2` the two arguments of a binary one,
 * each in its own range. It prints each input whose result differs from
 * MPFR's, at most 20, then how many there were. NaNs agree whatever their
 * bits, as which NaN isn't said. */
#include <math.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "oracle.h"

/* T's functions, exported by `exports.t`, which check.sh builds. */
#define EXPORT(name)                                                                               \
    double m_##name##_f64(double) __attribute__((weak));                                           \
    float m_##name##_f32(float) __attribute__((weak));
#define EXPORT2(name)                                                                              \
    double m_##name##_f64(double, double) __attribute__((weak));                                   \
    float m_##name##_f32(float, float) __attribute__((weak));
EXPORT(exp)
EXPORT(exp2)
EXPORT(exp_m1)
EXPORT(ln)
EXPORT(log2)
EXPORT(log10)
EXPORT(ln_1p)
EXPORT(sin)
EXPORT(cos)
EXPORT(tan)
EXPORT(asin)
EXPORT(acos)
EXPORT(atan)
EXPORT(sinh)
EXPORT(cosh)
EXPORT(tanh)
EXPORT(asinh)
EXPORT(acosh)
EXPORT(atanh)
EXPORT(cbrt)
EXPORT(sqrt)
EXPORT(sin_pi)
EXPORT(cos_pi)
EXPORT2(pow)
EXPORT2(atan2)
EXPORT2(hypot)

#define ENTRY(name) {#name, (void (*)(void))m_##name##_f64, (void (*)(void))m_##name##_f32}
static const struct {
    const char *name;
    void (*f64)(void);
    void (*f32)(void);
} T_FUNCTIONS[] = {
    ENTRY(exp),   ENTRY(exp2),  ENTRY(exp_m1), ENTRY(ln),    ENTRY(log2),  ENTRY(log10),
    ENTRY(ln_1p), ENTRY(sin),   ENTRY(cos),    ENTRY(tan),   ENTRY(asin),  ENTRY(acos),
    ENTRY(atan),  ENTRY(sinh),  ENTRY(cosh),   ENTRY(tanh),  ENTRY(asinh), ENTRY(acosh),
    ENTRY(atanh), ENTRY(cbrt),  ENTRY(sqrt),   ENTRY(pow),   ENTRY(atan2), ENTRY(hypot),
    ENTRY(sin_pi), ENTRY(cos_pi),
};

static int f;         /* the function, in FUNCTIONS */
static int type;      /* F32 or F64 */
static void (*t_fn)(void);

static uint64_t t_eval(uint64_t x, uint64_t y) {
    if (type == F64) {
        double r = arity(f) == 1 ? ((double (*)(double))t_fn)(of_bits64(x))
                                 : ((double (*)(double, double))t_fn)(of_bits64(x), of_bits64(y));
        return bits64(r);
    }
    float r = arity(f) == 1
                  ? ((float (*)(float))t_fn)(of_bits32((uint32_t)x))
                  : ((float (*)(float, float))t_fn)(of_bits32((uint32_t)x), of_bits32((uint32_t)y));
    return bits32(r);
}

static int is_nan(uint64_t b) {
    return type == F64 ? (b & 0x7FFFFFFFFFFFFFFFu) > 0x7FF0000000000000u
                       : (b & 0x7FFFFFFFu) > 0x7F800000u;
}

static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static uint64_t wrong;

/* A fast path, exported for `error`: its value is `m_core_hi(x) +
 * m_core_lo(x)`, scaled. */
double m_core_hi(double) __attribute__((weak));
double m_core_lo(double) __attribute__((weak));
static int measuring;
static double worst = -1000; /* log2 of the greatest relative error seen */
static uint64_t worst_x;

/* How far the fast path's `hi + lo` is from `f(x)`, relatively, as log2. */
static void measure(uint64_t xb) {
    double x = of_bits64(xb);
    double hi = m_core_hi(x), lo = m_core_lo(x);
    mpfr_set_emin(mpfr_get_emin_min());
    mpfr_set_emax(mpfr_get_emax_max());
    mpfr_t mx, exact, got;
    mpfr_inits2(300, mx, exact, got, (mpfr_ptr)0);
    mpfr_set_d(mx, x, MPFR_RNDN);
    FUNCTIONS[f].f1(exact, mx, MPFR_RNDN);
    mpfr_set_d(got, hi, MPFR_RNDN);
    mpfr_add_d(got, got, lo, MPFR_RNDN);
    mpfr_sub(got, got, exact, MPFR_RNDN);
    mpfr_div(got, got, exact, MPFR_RNDN);
    mpfr_abs(got, got, MPFR_RNDN);
    double e = mpfr_zero_p(got) ? -1000 : mpfr_get_exp(got) - 1 + log2(2 * mpfr_get_d_2exp(&(long){0}, got, MPFR_RNDN));
    mpfr_clears(mx, exact, got, (mpfr_ptr)0);
    if (mpfr_regular_p(exact) == 0) return;
    pthread_mutex_lock(&lock);
    if (e > worst) {
        worst = e;
        worst_x = xb;
    }
    pthread_mutex_unlock(&lock);
}

static void compare(uint64_t x, uint64_t y) {
    if (measuring) {
        measure(x);
        return;
    }
    uint64_t got = t_eval(x, y), want = oracle(f, type, x, y);
    if (got == want || (is_nan(got) && is_nan(want))) return;
    pthread_mutex_lock(&lock);
    if (wrong++ < 20) {
        if (arity(f) == 1)
            printf("%s(%#llx) = %#llx, not %#llx\n", FUNCTIONS[f].name, (unsigned long long)x,
                   (unsigned long long)got, (unsigned long long)want);
        else
            printf("%s(%#llx, %#llx) = %#llx, not %#llx\n", FUNCTIONS[f].name,
                   (unsigned long long)x, (unsigned long long)y, (unsigned long long)got,
                   (unsigned long long)want);
        fflush(stdout);
    }
    pthread_mutex_unlock(&lock);
}

/* splitmix64 */
static uint64_t next(uint64_t *s) {
    uint64_t z = (*s += 0x9E3779B97F4A7C15u);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9u;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBu;
    return z ^ (z >> 31);
}

static double uniform(uint64_t *s, double lo, double hi) {
    return lo + (hi - lo) * ((double)(next(s) >> 11) * 0x1p-53);
}

static uint64_t of_double(double d) { return type == F64 ? bits64(d) : bits32((float)d); }

enum { ALL, BITS, RANGE, RANGE2 };
static int mode;
static uint64_t count, seed;
static double lo, hi, lo2, hi2;
static int threads;

static void *work(void *arg) {
    uint64_t me = (uint64_t)(uintptr_t)arg;
    if (mode == ALL) {
        uint64_t per = (1ull << 32) / threads;
        uint64_t start = me * per, end = me + 1 == (uint64_t)threads ? 1ull << 32 : start + per;
        for (uint64_t x = start; x < end; x++) compare(x, 0);
        return 0;
    }
    uint64_t s = seed * 0x100000001B3u + me;
    uint64_t mask = type == F64 ? ~0ull : 0xFFFFFFFFu;
    for (uint64_t i = me; i < count; i += threads) {
        uint64_t x, y = 0;
        if (mode == BITS) {
            x = next(&s) & mask;
            y = next(&s) & mask;
        } else {
            x = of_double(uniform(&s, lo, hi));
            if (mode == RANGE2) y = of_double(uniform(&s, lo2, hi2));
        }
        compare(x, y);
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: check <fn> f32|f64 all|bits|range|range2|list ...\n");
        return 2;
    }
    f = function_named(argv[1]);
    if (f < 0) {
        fprintf(stderr, "no function %s\n", argv[1]);
        return 2;
    }
    type = strcmp(argv[2], "f64") == 0 ? F64 : F32;
    t_fn = type == F64 ? T_FUNCTIONS[f].f64 : T_FUNCTIONS[f].f32;
    if (!t_fn) {
        fprintf(stderr, "T has no %s of an %s\n", argv[1], argv[2]);
        return 2;
    }
    threads = getenv("THREADS") ? atoi(getenv("THREADS")) : 32;
    const char *m = argv[3];
    int a = 4;
    /* `error ...`: how far the fast path is, at most, of the inputs the
     * rest of the arguments say. */
    if (strcmp(m, "error") == 0) {
        if (!m_core_hi || !m_core_lo || type != F64) {
            fprintf(stderr, "error needs m_core_hi and m_core_lo, of an f64\n");
            return 2;
        }
        measuring = 1;
        m = argv[a++];
    }
    /* `bench lo hi`: nanoseconds a call, T's against the C library's. */
    if (strcmp(m, "bench") == 0 && arity(f) == 1) {
        double l = atof(argv[a]), h = atof(argv[a + 1]);
        enum { N = 1 << 22 };
        static double xs[N];
        static float fs[N];
        uint64_t st = 12345;
        for (int i = 0; i < N; i++) {
            xs[i] = uniform(&st, l, h);
            fs[i] = (float)xs[i];
        }
        double (*c64)(double) = 0;
        float (*c32)(float) = 0;
        static const struct { const char *n; double (*d)(double); float (*s)(float); } LIBM[] = {
            {"exp", exp, expf},     {"exp2", exp2, exp2f},     {"exp_m1", expm1, expm1f},
            {"ln", log, logf},      {"log2", log2, log2f},     {"log10", log10, log10f},
            {"ln_1p", log1p, log1pf}, {"sin", sin, sinf},     {"cos", cos, cosf},
            {"tan", tan, tanf},     {"asin", asin, asinf},     {"acos", acos, acosf},
            {"atan", atan, atanf},  {"sinh", sinh, sinhf},     {"cosh", cosh, coshf},
            {"tanh", tanh, tanhf},  {"asinh", asinh, asinhf},  {"acosh", acosh, acoshf},
            {"atanh", atanh, atanhf}, {"cbrt", cbrt, cbrtf},   {"sqrt", sqrt, sqrtf},
        };
        for (unsigned i = 0; i < sizeof LIBM / sizeof LIBM[0]; i++)
            if (strcmp(LIBM[i].n, argv[1]) == 0) {
                c64 = LIBM[i].d;
                c32 = LIBM[i].s;
            }
        for (int round = 0; round < 2; round++) {
            struct timespec t0, t1;
            volatile double sink = 0;
            double sum = 0;
            clock_gettime(CLOCK_MONOTONIC, &t0);
            if (type == F64)
                for (int i = 0; i < N; i++) sum += ((double (*)(double))t_fn)(xs[i]);
            else
                for (int i = 0; i < N; i++) sum += ((float (*)(float))t_fn)(fs[i]);
            clock_gettime(CLOCK_MONOTONIC, &t1);
            double tt = ((t1.tv_sec - t0.tv_sec) * 1e9 + (t1.tv_nsec - t0.tv_nsec)) / N;
            clock_gettime(CLOCK_MONOTONIC, &t0);
            if (type == F64)
                for (int i = 0; i < N; i++) sum += c64(xs[i]);
            else
                for (int i = 0; i < N; i++) sum += c32(fs[i]);
            clock_gettime(CLOCK_MONOTONIC, &t1);
            double tc = ((t1.tv_sec - t0.tv_sec) * 1e9 + (t1.tv_nsec - t0.tv_nsec)) / N;
            sink = sum;
            (void)sink;
            if (round == 1)
                printf("%s %s: T %.1f ns, C library %.1f ns a call\n", argv[1], argv[2], tt, tc);
        }
        return 0;
    }
    if (strcmp(m, "list") == 0) {
        char line[256];
        uint64_t n = 0;
        while (fgets(line, sizeof line, stdin)) {
            unsigned long long x = 0, y = 0;
            if (sscanf(line, "%llx %llx", &x, &y) < 1) continue;
            compare(x, y);
            n++;
        }
        printf("%s %s: %llu wrong of %llu\n", argv[1], argv[2], (unsigned long long)wrong,
               (unsigned long long)n);
        return wrong != 0;
    }
    if (strcmp(m, "all") == 0) {
        mode = ALL;
        count = 1ull << 32;
    } else if (strcmp(m, "bits") == 0) {
        mode = BITS;
        count = strtoull(argv[a++], 0, 10);
    } else if (strcmp(m, "range") == 0) {
        mode = RANGE;
        lo = atof(argv[a++]);
        hi = atof(argv[a++]);
        count = strtoull(argv[a++], 0, 10);
    } else if (strcmp(m, "range2") == 0) {
        mode = RANGE2;
        lo = atof(argv[a++]);
        hi = atof(argv[a++]);
        lo2 = atof(argv[a++]);
        hi2 = atof(argv[a++]);
        count = strtoull(argv[a++], 0, 10);
    } else {
        fprintf(stderr, "no mode %s\n", m);
        return 2;
    }
    seed = a < argc ? strtoull(argv[a], 0, 10) : 1;
    pthread_t ts[256];
    for (int i = 0; i < threads; i++) pthread_create(&ts[i], 0, work, (void *)(uintptr_t)i);
    for (int i = 0; i < threads; i++) pthread_join(ts[i], 0);
    if (measuring) {
        printf("%s: greatest error 2^%.2f, at %#llx (%a)\n", argv[1], worst,
               (unsigned long long)worst_x, of_bits64(worst_x));
        return 0;
    }
    printf("%s %s: %llu wrong of %llu\n", argv[1], argv[2], (unsigned long long)wrong,
           (unsigned long long)count);
    return wrong != 0;
}
