/* NHL Modding Studio — libspeex OS_SUPPORT_CUSTOM hooks (the upstream extension point in
 * libspeex/os_support.h; no upstream file is modified). Fatal errors unwind to the guarded shim
 * call instead of exit(1); warnings and notifications are silenced (Studio reports errors itself). */
#ifndef NHLMS_OS_SUPPORT_CUSTOM_H
#define NHLMS_OS_SUPPORT_CUSTOM_H

void nhlms_speex_fatal(const char *str, const char *file, int line);

#define OVERRIDE_SPEEX_FATAL
static inline void _speex_fatal(const char *str, const char *file, int line) {
    nhlms_speex_fatal(str, file, line);
}

#define OVERRIDE_SPEEX_WARNING
static inline void speex_warning(const char *str) {
    (void)str;
}

#define OVERRIDE_SPEEX_WARNING_INT
static inline void speex_warning_int(const char *str, int val) {
    (void)str;
    (void)val;
}

#define OVERRIDE_SPEEX_NOTIFY
static inline void speex_notify(const char *str) {
    (void)str;
}

#endif
