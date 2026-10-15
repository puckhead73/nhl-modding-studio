/* NHL Modding Studio — minimal C shim over upstream libspeex 1.2.1 (decoder only). See the header. */
#include <setjmp.h>
#include <stdlib.h>
#include <string.h>

#include "speex/speex.h"
#include "speex/speex_bits.h"
#include "nhlms_speex_shim.h"

#if defined(_MSC_VER)
#define NHLMS_TLS __declspec(thread)
#else
#define NHLMS_TLS _Thread_local
#endif

/* The largest packet the shim accepts: libspeex's own default bit-buffer size, so the buffer is
 * never reallocated or (on allocation failure) silently truncated by `speex_bits_read_from`. */
#define NHLMS_SPX_MAX_PACKET 2000

static NHLMS_TLS jmp_buf *fatal_jmp = NULL;
static NHLMS_TLS const char *fatal_msg = NULL;

/* Called by libspeex's `speex_fatal` / `speex_assert` (os_support_custom.h). Inside a guarded shim
 * call it unwinds to that call (C frames only); outside one it aborts, as upstream would exit. */
void nhlms_speex_fatal(const char *str, const char *file, int line) {
    (void)file;
    (void)line;
    fatal_msg = str;
    if (fatal_jmp) {
        longjmp(*fatal_jmp, 1);
    }
    abort();
}

struct nhlms_spx {
    SpeexBits bits;
    void *st;
    int frame_size;
    int poisoned;
};

nhlms_spx *nhlms_spx_new_uwb(int enhancer) {
    nhlms_spx *h = (nhlms_spx *)calloc(1, sizeof(nhlms_spx));
    jmp_buf env;
    jmp_buf *prev = fatal_jmp;
    int enh = enhancer ? 1 : 0;
    if (!h) {
        return NULL;
    }
    if (setjmp(env)) {
        fatal_jmp = prev;
        /* Partially initialised: release only what is known to exist; leak nothing reachable. */
        if (h->st) {
            speex_decoder_destroy(h->st);
        }
        free(h);
        return NULL;
    }
    fatal_jmp = &env;
    speex_bits_init(&h->bits);
    h->st = speex_decoder_init(speex_lib_get_mode(SPEEX_MODEID_UWB));
    if (!h->st) {
        fatal_jmp = prev;
        speex_bits_destroy(&h->bits);
        free(h);
        return NULL;
    }
    speex_decoder_ctl(h->st, SPEEX_SET_ENH, &enh);
    speex_decoder_ctl(h->st, SPEEX_GET_FRAME_SIZE, &h->frame_size);
    fatal_jmp = prev;
    return h;
}

int nhlms_spx_frame_size(const nhlms_spx *h) {
    if (!h) {
        return NHLMS_SPX_E_ARGS;
    }
    if (h->poisoned) {
        return NHLMS_SPX_E_POISONED;
    }
    return h->frame_size;
}

int nhlms_spx_decode(nhlms_spx *h, const unsigned char *data, int len, short *out, int out_cap) {
    jmp_buf env;
    jmp_buf *prev = fatal_jmp;
    int rc;
    if (!h || !data || !out || len < 1) {
        return NHLMS_SPX_E_ARGS;
    }
    if (h->poisoned) {
        return NHLMS_SPX_E_POISONED;
    }
    if (out_cap < h->frame_size) {
        return NHLMS_SPX_E_ARGS;
    }
    if (len > NHLMS_SPX_MAX_PACKET) {
        return NHLMS_SPX_E_OVERSIZE;
    }
    if (setjmp(env)) {
        fatal_jmp = prev;
        h->poisoned = 1;
        return NHLMS_SPX_E_FATAL;
    }
    fatal_jmp = &env;
    speex_bits_read_from(&h->bits, (const char *)data, len);
    rc = speex_decode_int(h->st, &h->bits, (spx_int16_t *)out);
    fatal_jmp = prev;
    if (rc == -1) {
        return NHLMS_SPX_E_END;
    }
    if (rc != 0) {
        return NHLMS_SPX_E_CORRUPT;
    }
    return h->frame_size;
}

int nhlms_spx_bits_remaining(nhlms_spx *h) {
    if (!h) {
        return NHLMS_SPX_E_ARGS;
    }
    if (h->poisoned) {
        return NHLMS_SPX_E_POISONED;
    }
    return speex_bits_remaining(&h->bits);
}

void nhlms_spx_free(nhlms_spx *h) {
    if (!h) {
        return;
    }
    /* After a fatal unwind the decoder's heap state is still owned by the handle; destroy frees
     * allocations only and runs no asserts. */
    if (h->st) {
        speex_decoder_destroy(h->st);
    }
    speex_bits_destroy(&h->bits);
    free(h);
}

const char *nhlms_spx_version(void) {
    const char *v = NULL;
    speex_lib_ctl(SPEEX_LIB_GET_VERSION_STRING, (void *)&v);
    return v;
}

const char *nhlms_spx_last_fatal(void) {
    return fatal_msg;
}
