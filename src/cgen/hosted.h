/* The hosted target (docs/39-freestanding.md): the platform's functions
 * from the C library, std's streams, and what's around `main`. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <signal.h>

void t_platform_write(uint8_t byte) {
    /* What the program wrote first, as the interpreter shows it. */
    fflush(stdout);
    fputc(byte, stderr);
}

_Noreturn void t_platform_halt(void) {
    fflush(stderr);
    _Exit(1);
}

/* `malloc` aligns for any type, which is all a T type needs: `align` is
 * at most `_Alignof(max_align_t)`. */
void *t_platform_alloc(size_t size, size_t align) {
    (void)align;
    return malloc(size);
}

void *t_platform_realloc(void *p, size_t size, size_t align, size_t new_size) {
    (void)size;
    (void)align;
    return realloc(p, new_size);
}

void t_platform_free(void *p, size_t size, size_t align) {
    (void)size;
    (void)align;
    free(p);
}

/* What a stream's intrinsic gives back for an error: 257 for a broken
 * pipe, 258 for anything else, as `std::io::error` reads it. */
static uint32_t t_io_error(void) { return errno == EPIPE ? 257 : 258; }

static uint32_t t_read_byte(void) {
    int c = fgetc(stdin);
    if (c != EOF) return (uint32_t)c;
    return ferror(stdin) ? t_io_error() : 256;
}

static uint32_t t_write_byte(FILE *f, uint8_t b) {
    errno = 0;
    return fputc(b, f) == EOF ? t_io_error() : 0;
}

/* Ends the program after `main` returned: flushes its output, and fails
 * if `failed`, as `t run` does; a failed flush is shown at `site`, where
 * `main` is. */
static int t_exit(bool failed, uint32_t site) {
    errno = 0;
    if (fflush(stdout) != 0) {
        int e = errno;
        t_begin();
        t_say("cannot flush stdout: ");
        t_say(strerror(e));
        t_say(" (os error ");
        t_say_i(e);
        t_say(")");
        t_end(site);
    }
    return failed ? 1 : 0;
}

static void t_start(void) {
#ifdef SIGPIPE
    signal(SIGPIPE, SIG_IGN);
#endif
}
