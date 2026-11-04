/* What `machine_calls.t` needs of the machine beyond `shared/machine.c`:
 * a C function that calls back into T, its export `tick`. */
#include <stdint.h>

void tick(void);

/* Calls the program's `tick`, `n` times. */
void tick_times(uint32_t n) {
    for (uint32_t i = 0; i < n; i++) tick();
}
