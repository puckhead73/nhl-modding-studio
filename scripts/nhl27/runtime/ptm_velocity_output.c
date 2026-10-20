/* Source velocity query packs unstripped components, then optionally converts
 * that packed result into the output coordinate frame. Skipped queries still
 * transform the existing output, rather than replacing it with zero. */
#include "ptm_native.h"
extern PTM_GAME_ABI void nhl27_ptm_velocity_output_transform(float*,const float*,const float*);
PTM_EXPORT void nhl27_ptm_velocity_output(float *out,const float *velocity,
    const panic_u8 *stripped,const float *transform,int write_velocity){
    unsigned index=0;
    if(write_velocity)for(unsigned lane=0;lane<3;lane++)if(!stripped[lane])out[index++]=velocity[lane];
    if(!transform)return;
    float unpacked[4]={0,0,0,0},converted[4];index=0;
    for(unsigned lane=0;lane<3;lane++)if(!stripped[lane])unpacked[lane]=out[index++];
    nhl27_ptm_velocity_output_transform(converted,unpacked,transform);
    index=0;
    for(unsigned lane=0;lane<3;lane++)if(!stripped[lane])out[index++]=converted[lane];
}
