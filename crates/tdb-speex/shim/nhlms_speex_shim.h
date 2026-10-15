/* NHL Modding Studio — minimal C shim over upstream libspeex 1.2.1 (decoder only).
 *
 * The shim owns every libspeex struct (SpeexBits, the decoder state) and every varargs-free control
 * call, and exposes an opaque handle with checked buffer operations, so the Rust side never mirrors
 * a C struct layout. libspeex's internal-assertion path (`speex_fatal`, upstream `exit(1)`) is
 * redirected through OS_SUPPORT_CUSTOM to a longjmp back into the guarded shim call; the handle is
 * then poisoned and refuses further use.
 */
#ifndef NHLMS_SPEEX_SHIM_H
#define NHLMS_SPEEX_SHIM_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct nhlms_spx nhlms_spx;

/* Result codes (negative); a successful decode returns the number of samples written. */
#define NHLMS_SPX_E_ARGS (-1)      /* null handle / buffer, len < 1, out_cap below the frame size */
#define NHLMS_SPX_E_END (-2)       /* libspeex returned -1: end-of-stream code in the packet */
#define NHLMS_SPX_E_CORRUPT (-3)   /* libspeex returned -2: corrupt stream (invalid mode etc.) */
#define NHLMS_SPX_E_FATAL (-4)     /* libspeex internal assertion; the handle is now poisoned */
#define NHLMS_SPX_E_POISONED (-5)  /* the handle hit a fatal error earlier and is unusable */
#define NHLMS_SPX_E_OVERSIZE (-6)  /* packet larger than the owned bit buffer (never truncated) */

/* A new ultra-wideband (32 kHz) decoder; `enhancer` 0 / 1 sets SPEEX_SET_ENH. NULL on failure. */
nhlms_spx *nhlms_spx_new_uwb(int enhancer);
/* Samples per decoded frame (640 for UWB), or a negative code. */
int nhlms_spx_frame_size(const nhlms_spx *h);
/* Decode exactly one packet into `out` (capacity `out_cap` samples). Returns the frame size or a
 * negative code. The packet is read from a copy owned by the handle; `data` is not retained. */
int nhlms_spx_decode(nhlms_spx *h, const unsigned char *data, int len, short *out, int out_cap);
/* Bits left unread in the last decoded packet (diagnostic), or a negative code. */
int nhlms_spx_bits_remaining(nhlms_spx *h);
/* Free the handle (NULL is a no-op). Safe on a poisoned handle. */
void nhlms_spx_free(nhlms_spx *h);
/* The upstream version string ("speex-1.2.1"). */
const char *nhlms_spx_version(void);
/* The message of the last fatal error on this thread, or NULL. */
const char *nhlms_spx_last_fatal(void);

#ifdef __cplusplus
}
#endif

#endif
