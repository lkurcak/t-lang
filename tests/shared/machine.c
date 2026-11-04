/* A machine for freestanding T programs that write their platform's
 * `t_platform_write` and `t_platform_halt` in T (docs/40-extern.md), for
 * the tests: memory from the C library, and a device the programs drive
 * with volatile accesses, whose functions they import.
 *
 * The device is a screen: bytes, and a cursor, at addresses the program
 * asks for. `power_off` shows what's on it, on `stdout`, and ends with the
 * code it's given. */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

void t_main(void);

static uint8_t screen[4096];
static uint64_t cursor;

uintptr_t screen_address(void) { return (uintptr_t)screen; }
uintptr_t screen_size(void) { return sizeof screen; }
uintptr_t cursor_address(void) { return (uintptr_t)&cursor; }

_Noreturn void power_off(int32_t code) {
    fwrite(screen, 1, cursor < sizeof screen ? cursor : sizeof screen, stdout);
    fflush(stdout);
    exit(code);
}

/* Adds two numbers, as a C function the program imports would. */
int64_t add(int64_t a, int64_t b) { return a + b; }
_Bool is_even(uint8_t n) { return n % 2 == 0; }

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

int main(void) {
    t_main();
    /* A program that returns from `main` hasn't powered off. */
    power_off(0);
}
