/* Writes the inputs tests/generated_math.rs checks, with MPFR's answers
 * (docs/102-math.md):
 *
 *     cc -O2 -pthread -Iscripts/math -o vectors scripts/math/vectors.c -lmpfr -lgmp
 *     ./vectors > tests/math/vectors.txt
 *
 * For each function and type: the inputs, among 300 million drawn at
 * random, whose exact values are nearest halfway between two floats, which
 * are those the fast paths can't decide and the slow paths do; some at
 * random; the edges; and `pow`'s exact and halfway values. A line is `name
 * type x [y] want`, in hex bits. The same seeds give the same file, in
 * about 25 minutes on 32 threads. */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

#include "oracle.h"

static uint64_t next(uint64_t *s) {
    uint64_t z = (*s += 0x9E3779B97F4A7C15u);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9u;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBu;
    return z ^ (z >> 31);
}

static double uniform(uint64_t *s, double lo, double hi) {
    return lo + (hi - lo) * ((double)(next(s) >> 11) * 0x1p-53);
}

/* Where each function's inputs are drawn from, half of them: the other
 * half are uniform bit patterns, every exponent alike. */
static const struct {
    const char *name;
    double lo, hi, lo2, hi2;
} RANGES[] = {
    {"exp", -745, 710},  {"exp2", -1075, 1024}, {"exp_m1", -40, 710}, {"ln", 0, 4},
    {"log2", 0, 4},      {"log10", 0, 4},       {"ln_1p", -1, 3},     {"sin", -20, 20},
    {"cos", -20, 20},    {"tan", -20, 20},      {"sinh", -712, 712},  {"cosh", -712, 712},
    {"tanh", -20, 20},   {"pow", 0, 4, -60, 60},
};
enum { N_RANGES = sizeof RANGES / sizeof RANGES[0] };

enum { KEEP = 24, RANDOM = 40, THREADS = 32 };

typedef struct {
    int h;
    uint64_t x, y;
} Hard;

static int f, type, range;
static uint64_t draws;
static Hard best[THREADS][KEEP];

static uint64_t to_bits(double d) { return type == F64 ? bits64(d) : bits32((float)d); }

static uint64_t draw(uint64_t *s, int second) {
    if (next(s) & 1) {
        uint64_t b = next(s);
        return type == F64 ? b : (b & 0xFFFFFFFFu);
    }
    double lo = second ? RANGES[range].lo2 : RANGES[range].lo;
    double hi = second ? RANGES[range].hi2 : RANGES[range].hi;
    return to_bits(uniform(s, lo, hi));
}

static void *search(void *arg) {
    uint64_t me = (uint64_t)(uintptr_t)arg;
    uint64_t s = 0x5EED0000u + me * 7919 + f * 104729 + type;
    Hard *keep = best[me];
    for (int i = 0; i < KEEP; i++) keep[i].h = -1;
    for (uint64_t i = me; i < draws; i += THREADS) {
        uint64_t x = draw(&s, 0), y = arity(f) == 2 ? draw(&s, 1) : 0;
        int h = hardness(f, type, x, y);
        int worst = 0;
        for (int k = 1; k < KEEP; k++)
            if (keep[k].h < keep[worst].h) worst = k;
        if (h > keep[worst].h) keep[worst] = (Hard){h, x, y};
    }
    return 0;
}

static void line(uint64_t x, uint64_t y) {
    uint64_t want = oracle(f, type, x, y);
    const char *t = type == F64 ? "f64" : "f32";
    if (arity(f) == 1)
        printf("%s %s %llx %llx\n", FUNCTIONS[f].name, t, (unsigned long long)x,
               (unsigned long long)want);
    else
        printf("%s %s %llx %llx %llx\n", FUNCTIONS[f].name, t, (unsigned long long)x,
               (unsigned long long)y, (unsigned long long)want);
}

static int by_hardness(const void *a, const void *b) {
    return ((const Hard *)b)->h - ((const Hard *)a)->h;
}

int main(int argc, char **argv) {
    draws = argc > 1 ? strtoull(argv[1], 0, 10) : 300000000;
    static const double EDGES[] = {0.0, -0.0, 1.0, -1.0, 0.5, 2.0, 10.0, 1e-300, -1e-300,
                                   5e-324, 1e300, -1e300, 0.1, 3.0, -3.0, 709.8, -745.2,
                                   88.7, -104.0, 1e22, 1048576.5};
    for (range = 0; range < N_RANGES; range++) {
        f = function_named(RANGES[range].name);
        for (type = F32; type <= F64; type++) {
            pthread_t ts[THREADS];
            for (int i = 0; i < THREADS; i++)
                pthread_create(&ts[i], 0, search, (void *)(uintptr_t)i);
            for (int i = 0; i < THREADS; i++) pthread_join(ts[i], 0);
            Hard all[THREADS * KEEP];
            for (int i = 0; i < THREADS; i++)
                for (int k = 0; k < KEEP; k++) all[i * KEEP + k] = best[i][k];
            qsort(all, THREADS * KEEP, sizeof all[0], by_hardness);
            for (int k = 0; k < KEEP; k++) line(all[k].x, all[k].y);
            uint64_t s = 0xACE + range * 31 + type;
            for (int k = 0; k < RANDOM; k++) {
                uint64_t x = draw(&s, 0), y = arity(f) == 2 ? draw(&s, 1) : 0;
                line(x, y);
            }
            int n = sizeof EDGES / sizeof EDGES[0];
            for (int i = 0; i < n; i++) {
                if (arity(f) == 1)
                    line(to_bits(EDGES[i]), 0);
                else
                    for (int j = 0; j < n; j += 3) line(to_bits(EDGES[i]), to_bits(EDGES[j]));
            }
            fprintf(stderr, "%s %s: hardest %d bits past the format's\n", RANGES[range].name,
                    type == F64 ? "f64" : "f32", all[0].h);
        }
    }
    /* `pow`'s exact values and halfway ones (lemma M4), which no random
     * draw finds: perfect powers, squares of odd integers of p/2 + 1 bits,
     * which are halfway between two floats, powers of two to the edge of
     * the subnormals, and -1 and 1. */
    f = function_named("pow");
    for (type = F32; type <= F64; type++) {
        int half = type == F64 ? 27 : 13;
        uint64_t s = 0x9E37 + type;
        for (int k = 0; k < 12; k++) {
            uint64_t r = ((next(&s) >> 40) % (1u << (half - 1))) | (1u << (half - 1)) | 1;
            line(to_bits((double)r), to_bits(2.0));
            line(to_bits((double)r * (double)r), to_bits(0.5));
            line(to_bits((double)r * (double)r), to_bits(1.5));
        }
        /* Cubes halfway between two floats: `r^3` of `p + 1` bits, `r` 3 mod
         * 4, so the even one is above, unlike a square's, whose is below:
         * rounding the value down by chance would get them wrong. */
        double lo = type == F64 ? 208064.0 : 256.0, span = type == F64 ? 54000.0 : 66.0;
        double top = type == F64 ? 0x1p54 : 0x1p25, bottom = top / 2;
        for (int k = 0, found = 0; k < 4000 && found < 12; k++) {
            uint64_t r = (uint64_t)(lo + (double)((next(&s) >> 11) % (uint64_t)span));
            r = (r & ~3ull) | 3;
            double cube = (double)r * (double)r * (double)r;
            if (cube < bottom || cube >= top) continue;
            line(to_bits((double)r), to_bits(3.0));
            line(to_bits(-(double)r), to_bits(3.0));
            if ((double)r * (double)r < top / 2) line(to_bits((double)r * (double)r), to_bits(1.5));
            found++;
        }
        static const double BASES[] = {9.0, 27.0, 3.0, 0.25, 6.25, 1e10, -2.0, -3.0, -1.0, 2.0, 0.5};
        static const double POWERS[] = {0.5, 1.0 / 3.0, 3.0, 2.0, 1.5, -1.0, -2.0, 0.25, 64.0, 33.0};
        for (unsigned i = 0; i < sizeof BASES / sizeof BASES[0]; i++)
            for (unsigned j = 0; j < sizeof POWERS / sizeof POWERS[0]; j++)
                line(to_bits(BASES[i]), to_bits(POWERS[j]));
        double edge = type == F64 ? -1075.0 : -150.0;
        line(to_bits(2.0), to_bits(edge));
        line(to_bits(2.0), to_bits(edge + 0.5));
        line(to_bits(2.0), to_bits(edge + 1.0));
        line(to_bits(0.5), to_bits(-edge));
        line(to_bits(4.0), to_bits(edge / 2));
        line(to_bits(-1.0), to_bits(9007199254740992.0));
        line(to_bits(-1.0), to_bits(1e300));
    }
    return 0;
}
