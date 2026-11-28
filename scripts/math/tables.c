/* Prints the constants and tables of core's math functions, as T, worked
 * out by MPFR at 400 bits and rounded once (docs/102-math.md); a table's
 * `f64`s as their bits, which are quicker to check than float literals:
 *
 *     cc -O2 -o tables scripts/math/tables.c -lmpfr -lgmp && ./tables <name>
 *
 * A double-double `hi + lo` is the `f64` nearest the value, then the one
 * nearest what's left: together within 2^-106 of it, relatively. Each is
 * written with 17 significant digits, which read back as that `f64`. */
#include <mpfr.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static mpfr_t v, w;

/* `x` as a T float literal: 17 significant digits, with a point. */
static const char *lit(double x) {
    static char buf[4][40];
    static int k;
    char *b = buf[k++ % 4];
    snprintf(b, 40, "%.17g", x);
    if (!strpbrk(b, ".en")) strcat(b, ".0");
    return b;
}

/* `x`'s bits, as T writes a `u64` literal. */
static void hex(double x) {
    uint64_t b;
    memcpy(&b, &x, 8);
    printf("0x%016llX, ", (unsigned long long)b);
}

static void one(const char *name) {
    double hi = mpfr_get_d(v, MPFR_RNDN);
    mpfr_sub_d(w, v, hi, MPFR_RNDN);
    double lo = mpfr_get_d(w, MPFR_RNDN);
    printf("%s: %s %s (%a %a)\n", name, lit(hi), lit(lo), hi, lo);
}

int main(int argc, char **argv) {
    mpfr_inits2(400, v, w, (mpfr_ptr)0);
    const char *t = argc > 1 ? argv[1] : "";
    if (strcmp(t, "exp2") == 0) {
        /* 2^(j/64), j from 0 to 63. */
        for (int j = 0; j < 64; j++) {
            mpfr_set_si(v, j, MPFR_RNDN);
            mpfr_div_ui(v, v, 64, MPFR_RNDN);
            mpfr_exp2(v, v, MPFR_RNDN);
            double hi = mpfr_get_d(v, MPFR_RNDN);
            mpfr_sub_d(w, v, hi, MPFR_RNDN);
            printf("    ");
            hex(hi);
            hex(mpfr_get_d(w, MPFR_RNDN));
            printf("\n");
        }
    } else if (strcmp(t, "ln1") == 0) {
        /* ln's first table, by the top 7 bits of the significand, `i`:
         * `m` in [1 + i/128, 1 + (i+1)/128), or half that from i = 53 on,
         * where m is at least sqrt(2) and is halved. `c`, the f64 nearest
         * 1 over the middle, or 1 where the interval holds 1, and -ln(c)
         * as hi + lo. */
        for (int i = 0; i < 128; i++) {
            double lo = 1 + i / 128.0, hi = 1 + (i + 1) / 128.0;
            if (i >= 53) {
                lo /= 2;
                hi /= 2;
            }
            mpfr_t c;
            mpfr_init2(c, 53);
            if (i == 0 || i == 127)
                mpfr_set_ui(c, 1, MPFR_RNDN);
            else {
                mpfr_set_d(v, (lo + hi) / 2, MPFR_RNDN);
                mpfr_ui_div(c, 1, v, MPFR_RNDN);
            }
            mpfr_log(v, c, MPFR_RNDN);
            mpfr_neg(v, v, MPFR_RNDN);
            double h = mpfr_get_d(v, MPFR_RNDN);
            mpfr_sub_d(w, v, h, MPFR_RNDN);
            printf("    ");
            hex(mpfr_get_d(c, MPFR_RNDN));
            hex(h);
            hex(mpfr_get_d(w, MPFR_RNDN));
            printf("\n");
            mpfr_clear(c);
        }
    } else if (strcmp(t, "ln2t") == 0) {
        /* ln's second table, by j from -40 to 72, of u near 1 + j/2^13:
         * c, the f64 nearest 1/(1 + j/2^13), and -ln(c). */
        for (int j = -40; j <= 72; j++) {
            mpfr_t c;
            mpfr_init2(c, 53);
            mpfr_set_si(v, j, MPFR_RNDN);
            mpfr_div_2ui(v, v, 13, MPFR_RNDN);
            mpfr_add_ui(v, v, 1, MPFR_RNDN);
            mpfr_ui_div(c, 1, v, MPFR_RNDN);
            mpfr_log(v, c, MPFR_RNDN);
            mpfr_neg(v, v, MPFR_RNDN);
            double h = mpfr_get_d(v, MPFR_RNDN);
            mpfr_sub_d(w, v, h, MPFR_RNDN);
            printf("    ");
            hex(mpfr_get_d(c, MPFR_RNDN));
            hex(h);
            hex(mpfr_get_d(w, MPFR_RNDN));
            printf("\n");
            mpfr_clear(c);
        }
    } else if (strcmp(t, "sincos") == 0) {
        /* sin(j/64) and cos(j/64), j from 0 to 51, each as hi + lo. */
        for (int j = 0; j < 52; j++) {
            mpfr_t s, c, a;
            mpfr_inits2(400, s, c, a, (mpfr_ptr)0);
            mpfr_set_ui(a, j, MPFR_RNDN);
            mpfr_div_ui(a, a, 64, MPFR_RNDN);
            mpfr_sin_cos(s, c, a, MPFR_RNDN);
            double sh = mpfr_get_d(s, MPFR_RNDN);
            mpfr_sub_d(s, s, sh, MPFR_RNDN);
            double ch = mpfr_get_d(c, MPFR_RNDN);
            mpfr_sub_d(c, c, ch, MPFR_RNDN);
            printf("    ");
            hex(sh);
            hex(mpfr_get_d(s, MPFR_RNDN));
            hex(ch);
            hex(mpfr_get_d(c, MPFR_RNDN));
            printf("\n");
            mpfr_clears(s, c, a, (mpfr_ptr)0);
        }
    } else if (strcmp(t, "atan") == 0) {
        /* atan(j/64), j from 0 to 64, as hi + lo. */
        for (int j = 0; j <= 64; j++) {
            mpfr_set_ui(v, j, MPFR_RNDN);
            mpfr_div_ui(v, v, 64, MPFR_RNDN);
            mpfr_atan(v, v, MPFR_RNDN);
            double hi = mpfr_get_d(v, MPFR_RNDN);
            mpfr_sub_d(w, v, hi, MPFR_RNDN);
            printf("    ");
            hex(hi);
            hex(mpfr_get_d(w, MPFR_RNDN));
            printf("\n");
        }
    } else if (strcmp(t, "bits") == 0 && argc > 3) {
        /* The bits of 2/pi or pi/2 after the point, `n` words of 32, the
         * first word's top bit the first bit after the point. */
        int n = atoi(argv[3]);
        mpfr_t x;
        mpfr_init2(x, 32 * n + 64);
        mpfr_const_pi(x, MPFR_RNDN);
        if (strcmp(argv[2], "2/pi") == 0)
            mpfr_ui_div(x, 2, x, MPFR_RNDN);
        else
            mpfr_div_ui(x, x, 2, MPFR_RNDN);
        mpfr_frac(x, x, MPFR_RNDN);
        for (int i = 0; i < n; i++) {
            mpfr_mul_2ui(x, x, 32, MPFR_RNDN);
            unsigned long w = mpfr_get_ui(x, MPFR_RNDZ);
            mpfr_sub_ui(x, x, w, MPFR_RNDN);
            printf("0x%08lX,%s", w, (i % 6 == 5) ? "\n" : " ");
        }
        printf("\n");
        mpfr_clear(x);
    } else if (strcmp(t, "split") == 0 && argc > 3) {
        /* A constant `c` as `hi + lo`, `hi` of `bits` bits, so that its
         * product with an integer of `53 - bits` bits is exact: ln2/64 at
         * 36. */
        int bits = atoi(argv[3]);
        if (strcmp(argv[2], "ln2/64") == 0) {
            mpfr_const_log2(v, MPFR_RNDN);
            mpfr_div_ui(v, v, 64, MPFR_RNDN);
        } else if (strcmp(argv[2], "pi/2") == 0) {
            mpfr_const_pi(v, MPFR_RNDN);
            mpfr_div_ui(v, v, 2, MPFR_RNDN);
        } else if (strcmp(argv[2], "ln2") == 0) {
            mpfr_const_log2(v, MPFR_RNDN);
        } else if (strcmp(argv[2], "pi/2-4") == 0) {
            /* pi/2 as four parts, three of `bits` bits, then one of 53. */
            mpfr_const_pi(v, MPFR_RNDN);
            mpfr_div_ui(v, v, 2, MPFR_RNDN);
            mpfr_t h;
            mpfr_init2(h, bits);
            for (int i = 0; i < 3; i++) {
                mpfr_set(h, v, MPFR_RNDN);
                double d = mpfr_get_d(h, MPFR_RNDN);
                printf("%s ", lit(d));
                mpfr_sub_d(v, v, d, MPFR_RNDN);
            }
            printf("%s\n", lit(mpfr_get_d(v, MPFR_RNDN)));
            return 0;
        } else {
            return 2;
        }
        mpfr_t h;
        mpfr_init2(h, bits);
        mpfr_set(h, v, MPFR_RNDN);
        double hi = mpfr_get_d(h, MPFR_RNDN);
        mpfr_sub_d(w, v, hi, MPFR_RNDN);
        double lo = mpfr_get_d(w, MPFR_RNDN);
        mpfr_sub_d(w, w, lo, MPFR_RNDN);
        double lo2 = mpfr_get_d(w, MPFR_RNDN);
        printf("%s = %s + %s + %s\n", argv[2], lit(hi), lit(lo), lit(lo2));
    } else if (strcmp(t, "consts") == 0) {
        mpfr_const_log2(v, MPFR_RNDN);
        one("ln2");
        mpfr_const_log2(v, MPFR_RNDN);
        mpfr_div_ui(v, v, 64, MPFR_RNDN);
        one("ln2/64");
        mpfr_const_log2(v, MPFR_RNDN);
        mpfr_ui_div(v, 64, v, MPFR_RNDN);
        one("64/ln2");
        mpfr_const_pi(v, MPFR_RNDN);
        one("pi");
        mpfr_const_pi(v, MPFR_RNDN);
        mpfr_div_ui(v, v, 2, MPFR_RNDN);
        one("pi/2");
        mpfr_const_pi(v, MPFR_RNDN);
        mpfr_ui_div(v, 2, v, MPFR_RNDN);
        one("2/pi");
        mpfr_set_ui(v, 10, MPFR_RNDN);
        mpfr_log2(v, v, MPFR_RNDN);
        one("log2(10)");
        mpfr_set_ui(v, 2, MPFR_RNDN);
        mpfr_log10(v, v, MPFR_RNDN);
        one("log10(2)");
        mpfr_set_ui(v, 1, MPFR_RNDN);
        mpfr_exp(v, v, MPFR_RNDN);
        mpfr_log2(v, v, MPFR_RNDN);
        one("log2(e)");
        mpfr_set_ui(v, 1, MPFR_RNDN);
        mpfr_exp(v, v, MPFR_RNDN);
        mpfr_log10(v, v, MPFR_RNDN);
        one("log10(e)");
    } else {
        fprintf(stderr, "usage: tables exp2|consts\n");
        return 2;
    }
    return 0;
}
