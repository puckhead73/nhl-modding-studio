/* Source 23a8d0/250f50, using native linked interfaces and path storage. */
#include "ptm_blend_storage.h"
typedef panic_address A;
typedef panic_u8 B;
#define ABI PTM_GAME_ABI
struct vector3 {float x,y,z;};
extern ABI void nhl27_ptm_sample_path(float*,void*,float);
extern void *nhl24_ptm_selected_path(void*,void*);

PTM_EXPORT void nhl24_ptm_blend_warp(void *match,void *context,void *extra_path,
        float remaining,float (ABI *remainder)(float,float)){
    B *m=match,*warp=*(B**)(*(B**)(m+0x11f0)+0xa8);
    void *driver=*(void**)(warp+0x18);
    B enabled=driver?((B(ABI*)(void*,void*))(*(A**)driver)[3])(driver,context):warp[0xd0];
    if(!enabled)return;
    float sample[4];
    nhl27_ptm_sample_path(sample,nhl24_ptm_selected_path(match,extra_path),remaining);
    /* Re-read after the Bool callback, matching source's parameter reload. */
    warp=*(B**)(*(B**)(m+0x11f0)+0xa8);
    driver=*(void**)(warp+8);
    struct vector3 position={sample[0],sample[1],sample[2]};
    if(driver)position=((struct vector3(ABI*)(void*,void*))(*(A**)driver)[3])(driver,context);
    driver=*(void**)(warp+0x10);
    float facing=driver?((float(ABI*)(void*,void*))(*(A**)driver)[3])(driver,context):sample[3];
    union {float f;unsigned u;} signed_pi={facing};
    signed_pi.u=(signed_pi.u&0x80000000u)|0x40490fdbu;
    facing=remainder(facing+signed_pi.f,0x1.921fb6p+2f)-signed_pi.f;
    if(0x1.921fb6p+1f<=facing)facing=-0x1.921fb6p+1f;
    float time=remaining**(float*)(m+0x88);
    float *target=(float*)(m+0x1170);
    target[0]=position.x;target[1]=position.y;target[2]=position.z;target[3]=facing;
    for(unsigned n=0;n<6;n++)((float*)(m+0x1180))[n]=time;
    m[0x1198]=0;
}
PTM_EXPORT ABI void nhl24_ptm_blend_warp_guest(A base,void *match,void *context,void *extra_path,float remaining){
    /* Source PLT bfabf50 resolves to 88Vv-AzHVj8 (fmodf). Native 7519d80
     * implements the same binary32 remainder with sign-preserving zero. */
    nhl24_ptm_blend_warp(match,context,extra_path,remaining,(float(ABI*)(float,float))(base+0x7519d80));
}
