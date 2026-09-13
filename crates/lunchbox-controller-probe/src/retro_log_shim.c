#include <stdarg.h>
#include <stdio.h>

/*
 * Libretro's logging callback is C-variadic, which Rust cannot define on all
 * supported stable targets. Keep the ABI boundary in this tiny C shim and
 * preserve trusted-core diagnostics on stderr.
 */
void lunchbox_retro_log(int level, const char *format, ...) {
    if (format == NULL) {
        return;
    }

    fprintf(stderr, "[libretro:%d] ", level);
    va_list arguments;
    va_start(arguments, format);
    vfprintf(stderr, format, arguments);
    va_end(arguments);
    fflush(stderr);
}
