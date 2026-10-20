/* Move-specific selection and puck release. Native gesture recognition and
 * gameplay values remain NHL24-supported; source-only variants stay private.
 * Evidence: source 47dea20, 47e55e0, 47e0810; native 2b611c0, 2b605a0.
 */
typedef __UINTPTR_TYPE__ address;
typedef unsigned long long u64;
typedef unsigned int u32;
typedef unsigned char u8;
#define ABI __attribute__((sysv_abi))
#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((visibility("default")))
#endif

/* Angles here are unsigned separations in degrees, except velocity_delta,
 * which is the wrapped velocity-heading minus facing-heading in radians.
 * Threshold defaults come from source initialization 47f907..47f9f8.
 */
EXPORT unsigned deke_high_level(unsigned native, float destination_facing,
        float destination_velocity, float facing_velocity, float velocity_delta,
        unsigned mirrored) {
    if (!native || native == 2) return native;
    if (destination_facing > 45.0f && destination_facing < 95.0f &&
            destination_velocity <= 20.0f) {
        return ((velocity_delta > 3.1415927410125732421875f) != !!mirrored) ? 5 : 4;
    }
    return facing_velocity < 130.0f ? 1 : 3;
}

EXPORT unsigned deke_toe_type(unsigned high, unsigned native_type) {
    if (native_type != 5 && native_type != 15) return native_type;
    if (high == 4) return 26;
    if (high == 5) return 27;
    if (high == 3) return 25;
    return native_type;
}

EXPORT unsigned deke_toe_secondary(unsigned high, unsigned secondary,
        float requested_local_move_angle) {
    if (high >= 1 && high <= 3) {
        if (requested_local_move_angle > 0.0f && secondary == 1) return 5;
        if (!(requested_local_move_angle > 0.0f) && secondary == 3) return 6;
    }
    return secondary;
}

EXPORT unsigned deke_bank_type(unsigned native, float stick_facing_separation) {
    /* Source compares the camera-relative left-stick angle against facing.
     * Behind-net rows 5/6 keep their native selection. Reverse board banks
     * extend 1/2 to 7/8; reverse mid-ice banks extend 3/4 to 9/10.
     */
    if (native >= 1 && native <= 4 &&
            stick_facing_separation > 2.35619449615478515625f) return native + 6;
    return native;
}

#define PI 3.1415927410125732421875f
#define TAU 6.283185482025146484375f
static float wrapped(float a) {
    /* Input headings and decoded pad angles are bounded. Reject corrupt values
     * instead of looping indefinitely on NaN/infinity or an invalid actor. */
    if (!(a > -100.0f && a < 100.0f)) return 0.0f;
    while (a < 0.0f) a += TAU;
    while (a >= TAU) a -= TAU;
    return a;
}
static float separation(float a) {
    a = wrapped(a); return a > PI ? TAU-a : a;
}
static float sine(float a) {
    double x = wrapped(a); if (x > PI) x -= TAU;
    double y=x*x;
    return (float)(x*(1.0+y*(-1.0/6+y*(1.0/120+y*(-1.0/5040+
        y*(1.0/362880+y*(-1.0/39916800+y*(1.0/6227020800+
        y*(-1.0/1307674368000+y/355687428096000)))))))));
}
static float vector_angle(const float *a, const float *b) {
    float aa=a[0]*a[0]+a[1]*a[1]+a[2]*a[2];
    float bb=b[0]*b[0]+b[1]*b[1]+b[2]*b[2];
    if (aa < 1e-12f || bb < 1e-12f) return 0.0f;
    float x=(a[0]*b[0]+a[1]*b[1]+a[2]*b[2])/
        (__builtin_sqrtf(aa)*__builtin_sqrtf(bb));
    if (x >= 1.0f) return 0.0f;
    if (x < -1.0f) return PI;
    float ax=x < 0.0f ? -x : x;
    /* Original source acos approximation, constants from 47dea20. */
    float p=ax*(ax*(ax*(ax*(ax*(ax*(ax*-0x1.4af458p-10f+
        0x1.b5218ap-8f)-0x1.17f8ccp-6f)+0x1.fa21f0p-6f)-
        0x1.9b0724p-5f)+0x1.6c753cp-4f)-0x1.b77f94p-3f)+0x1.921fb4p+0f;
    float result=__builtin_sqrtf(1.0f-ax)*p;
    return x < 0.0f ? PI-result : result;
}

extern ABI int deke_ant_bind_guest(address,void*,u64,void**);
static void publish(address base, void *context, u64 id, unsigned value) {
    unsigned *slot=0;
    if (!deke_ant_bind_guest(base,context,id,(void**)&slot)) *slot=value;
}
static void *pointer(void *p,unsigned offset) {return *(void**)((u8*)p+offset);}
static float scalar(void *p,unsigned offset) {return *(float*)((u8*)p+offset);}
static unsigned value(void *record,unsigned offset) {
    unsigned *p=pointer(record,offset); return p ? *p : 0;
}
static void *context_for(void *actor) {
    if (!actor) return 0;
    void *animation=pointer(actor,0x60);
    if (!animation) return 0;
    void *owner=pointer(animation,0xc28);
    if (!owner) return 0;
    void *context=pointer(owner,0x20);
    return context ? (u8*)context+0x60 : 0;
}
static const u64 velocity_ids[3]={0xd0de771f4db32fe6ULL,
    0x18f7fe05f9f5c2bbULL,0xc24f7e4dafbce10fULL};

/* Source 47e62ce captures velocity immediately after accepting the bank
 * command, before movement initialization changes the destination. */
EXPORT ABI void deke_bank_capture_guest(address base,void *owner) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    void *pose=pointer(actor,0x10);
    if (!pose) return;
    for (unsigned i=0;i<3;++i)
        publish(base,context,velocity_ids[i],*(unsigned*)((u8*)pose+0x50+4*i));
}
static void *record_for(address base,void *context,unsigned offset,address getter) {
    void *wrapper=((void*(ABI*)(void*))(base+0x2ec80c0))((u8*)context-0x60);
    if (!wrapper) return 0;
    void *root=pointer(wrapper,0x10);
    if (!root || !pointer(root,offset)) return 0;
    return ((void*(ABI*)(void*,void*))(base+getter))(pointer(root,offset),context);
}

static unsigned classify(address base,void *actor,unsigned native) {
    if (!native || native==2) return native;
    void *pose=pointer(actor,0x10),*motion=pointer(actor,0x70);
    if (!pose || !motion) return native;
    float heading=scalar(pose,0x84),delta=wrapped(scalar(pose,0x88)-heading);
    float destination[3], facing[3]={sine(heading),0,sine(heading+PI/2)};
    for (unsigned i=0;i<3;++i)
        destination[i]=scalar(motion,0x50+4*i)-scalar(pose,0x30+4*i);
    unsigned mirrored=((u8(ABI*)(void*))(*(address**)actor)[18])(actor);
    return deke_high_level(native,
        vector_angle(destination,facing)*57.295780181884765625f,
        vector_angle(destination,(float*)((u8*)pose+0x50))*57.295780181884765625f,
        separation(delta)*57.295780181884765625f,delta,mirrored);
}

/* Invoked after NHL24 has produced high-level state, before native consumers.
 * No native flag, threshold, controller property, or move owner is changed. */
EXPORT ABI void deke_high_guest(address base,void *owner) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    void *enums=record_for(base,context,0x90,0x4076f0);
    if (!enums) return;
    publish(base,context,0x3bccf76887b8b101ULL,classify(base,actor,value(enums,0x230)));
    if (!value(enums,0x238)) publish(base,context,0xabcc521ace57de1cULL,0);
}

/* Invoked at the two existing toe-drag setter calls. Native caller has already
 * recognized the Skill Stick command and computed LR/MR/RL/ML and speed class. */
EXPORT ABI void deke_toe_guest(address base,void *owner,unsigned type,unsigned secondary) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    void *enums=record_for(base,context,0x90,0x4076f0);
    void *floats=record_for(base,context,0x88,0x408710);
    if (!enums || !floats) return;
    unsigned *sample=0;
    unsigned high=classify(base,actor,value(enums,0x230));
    if (!deke_ant_bind_guest(base,context,0x3bccf76887b8b101ULL,(void**)&sample))
        high=*sample;
    /* Native stationary-toe recognition changes its public high-level value
     * to 2. Preserve the earlier lateral sample; apply that promotion only to
     * source forward motion, as 47e55e0 does. */
    if (high==1 && type==15) high=2;
    float *local=pointer(floats,0x1c0);
    publish(base,context,0x68f19d1c486bd2f0ULL,deke_toe_type(high,type));
    publish(base,context,0x36cb1e46bd675c4aULL,
        deke_toe_secondary(high,secondary,local ? *local : 0.0f));
}

/* Source 47e9d7a clears the request after the toe animation starts. Mirror
 * NHL24's corresponding clear so the private selector cannot keep requesting
 * a stale variant while the animation transitions out. */
EXPORT ABI void deke_toe_clear_guest(address base,void *owner) {
    void *context=context_for(pointer(owner,0x58));
    if (context) publish(base,context,0x68f19d1c486bd2f0ULL,0);
}

/* Promote only a recognized reverse board situation to the matching native
 * net-reversal gameplay type. Both values are supported by NHL24. The private
 * 7/8 context remains available to choose the board rebound at puck release.
 * This shares the whole move, without writing skating velocity or pose. */
EXPORT unsigned deke_bank_gameplay_type(unsigned native, unsigned variant) {
    if (native == 1 && variant == 7) return 6; /* backhand */
    if (native == 2 && variant == 8) return 5; /* forehand */
    if (native == 3 && variant == 9) return 5; /* open-ice forehand */
    if (native == 4 && variant == 10) return 6; /* open-ice backhand */
    return native;
}

/* Pickup query 2c0ff6a chooses dedicated board-hand pickup filters for 1/2,
 * generic filters for 3..6. Keep the net type for movement and rebound, but
 * restore the board situation only in this query's local value. */
EXPORT unsigned deke_bank_pickup_type(unsigned native,unsigned variant) {
    if (native==6 && variant==7) return 1;
    if (native==5 && variant==8) return 2;
    return native;
}
EXPORT ABI void deke_bank_pickup_guest(address base,void *owner,unsigned *native) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    unsigned *variant=0;
    if (!deke_ant_bind_guest(base,context,0xabcc521ace57de1cULL,(void**)&variant) && variant)
        *native=deke_bank_pickup_type(*native,*variant);
}

/* After the preferred pickup fails, native 2c101e0 waits for the move's
 * specific-pickup window before trying the general stick pickup. Let these
 * board and open-ice reversals try that fallback after 6 ticks;
 * all collision/possession and
 * animation eligibility checks remain native. Never lengthen a shorter wait. */
EXPORT unsigned deke_pickup_wait(unsigned native,unsigned variant,unsigned ticks) {
    if (((native==6 && variant==7) || (native==5 && variant==8)) &&
            (int)ticks>6) return 6;
    if (((native==6 && variant==10) || (native==5 && variant==9)) &&
            (int)ticks>6) return 6;
    return ticks;
}
EXPORT ABI void deke_pickup_wait_guest(address base,void *actor,unsigned *ticks) {
    void *context=context_for(actor);
    if (!context) return;
    unsigned *variant=0;
    if (deke_ant_bind_guest(base,context,0xabcc521ace57de1cULL,(void**)&variant) || !variant) return;
    void *enums=record_for(base,context,0x90,0x4076f0);
    if (enums) *ticks=deke_pickup_wait(value(enums,0x238),*variant,*ticks);
}

EXPORT ABI void deke_bank_guest(address base,void *owner) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    void *enums=record_for(base,context,0x90,0x4076f0);
    if (!enums) return;
    unsigned bank=value(enums,0x238);
    unsigned *previous=0;
    if (deke_ant_bind_guest(base,context,0xabcc521ace57de1cULL,(void**)&previous) ||
            !previous) return;
    if (bank && *previous) return;
    signed char controller=*(signed char*)((u8*)actor+0x23c);
    if (bank>=1 && bank<=4 && controller>=0) {
        /* Same controller lookup used by native 2b63270. Angle decoder 5003fb0
         * matches source 69a8ac0(current sample, camera-relative), the producer
         * of source analog action 7 at 4b47f08. No new pad interpretation. */
        unsigned index=(unsigned)controller;
        void *pool=*(void**)(base+0xba19f48);
        void **manager_slot=*(void***)(base+0xba1a418);
        if (pool && manager_slot && *manager_slot) {
            void *row=(u8*)pool+(index/6)*0x2a8+(index%6)*0x70;
            void *manager=*manager_slot;
            void *packet=((void*(ABI*)(void*,void*,u32))(*(address**)manager)[23])
                (manager,pointer(row,0),*(u32*)((u8*)row+8));
            if (packet) {
                float angle=0;
                ((void(ABI*)(float*,void*,int))(base+0x5003fb0))(&angle,packet,1);
                bank=deke_bank_type(bank,separation(angle-scalar(pointer(actor,0x10),0x84)));
            }
        }
    }
    unsigned *native_slot=pointer(enums,0x238);
    if (native_slot) *native_slot=deke_bank_gameplay_type(*native_slot,bank);
    publish(base,context,0xabcc521ace57de1cULL,bank);
}

/* Source 47e1430 reverse-near-boards branch. The board tangent is chosen
 * against facing, reversed, then biased into the boards using the original
 * speed-to-angle curve. Merely selecting its animation leaves a forward shot.
 */
static float heading_of(float x,float z) {
    float ax=x<0?-x:x,az=z<0?-z:z;
    if (ax<=0x1p-16f && az<=0x1p-16f) return 0;
    float hi=ax>az?ax:az,lo=ax<az?ax:az,t=lo/hi,y=t*t;
    float a=t*(y*(y*(y*(y*(y*-0x1.b9ba6p-7f+0x1.d6daaap-5f)-
        0x1.f09862p-4f)+0x1.90a992p-3f)-0x1.54fc8ap-2f)+0x1.ffff6ep-1f);
    if (ax>az) a=PI/2-a;
    if (z<0) a=PI-a;
    if (x<0) a=-a;
    return wrapped(a);
}
static float curve(float x,float xmax,float low,float high) {
    x=x<0?0:(x>xmax?xmax:x);
    return low+(high-low)*(x/xmax);
}
EXPORT void deke_reverse_board_velocity(float *out,float normal_x,float normal_z,
        float facing,float skater_speed,float board_distance) {
    float normal=heading_of(normal_x,normal_z);
    float tangent=wrapped(normal+PI/2);
    if (separation(tangent-facing)>PI/2) tangent=wrapped(normal+3*PI/2);
    float reverse=wrapped(tangent+PI),side=wrapped(normal-reverse);
    /* Source defaults at ef20e40/ef20e50; both are clamped interpolators. */
    float offset=curve(skater_speed,0.4f,0.75f,1.4f);
    if (side>=0 && side<PI) offset=wrapped(-offset);
    float angle=wrapped(reverse+offset);
    float speed=curve(board_distance,4.0f,0.3f,0.4f);
    out[0]=sine(angle)*speed;out[1]=0;
    out[2]=sine(angle+PI/2)*speed;out[3]=0;
}

EXPORT void deke_reverse_midice_velocity(float *out,const float *initial,
        const float *left,const float *right,const float *puck) {
    float direction[3],velocity[3],norm=0,speed=0;
    for (unsigned i=0;i<3;++i) {
        direction[i]=(left[i]+right[i])*0.5f-puck[i];
        velocity[i]=-initial[i]*0.3f;
        norm+=direction[i]*direction[i];speed+=velocity[i]*velocity[i];
    }
    norm=__builtin_sqrtf(norm);speed=__builtin_sqrtf(speed);
    /* 69bc4f0 leaves tiny vectors unchanged (squared cutoff 2^-32). */
    float reciprocal=norm>0x1p-16f ? 1.0f/norm : 1.0f;
    for (unsigned i=0;i<3;++i)
        out[i]=direction[i]*reciprocal*speed*0.6f+velocity[i]*(1.0f-0.6f);
    out[1]=0;out[3]=0;
    float result=__builtin_sqrtf(out[0]*out[0]+out[2]*out[2]);
    if (result<0.12f) {
        reciprocal=result>0x1p-16f ? 1.0f/result : 1.0f;
        out[0]=out[0]*reciprocal*0.12f;out[2]=out[2]*reciprocal*0.12f;
    }
}

/* A firmer ground drop: target 0.90 m behind the release pose over 12 ticks.
 * A 0.075 floor prevents a nearby release point producing a near-stationary
 * puck; the 0.11 cap keeps sprinting from turning this into a hard back pass.
 * Apply the requested 1.5x power increase after the original clamp, so every
 * nonzero drop gains exactly 50% launch velocity without changing direction.
 * These are launch tuning values, not a guaranteed friction-adjusted endpoint. */
EXPORT void deke_soft_drop_velocity(float *out,const float *position,
        float facing,const float *puck) {
    float x=position[0]-0.90f*sine(facing)-puck[0];
    float z=position[2]-0.90f*sine(facing+PI/2)-puck[2];
    float distance=__builtin_sqrtf(x*x+z*z);
    if (distance<0x1p-16f) { out[0]=out[1]=out[2]=out[3]=0;return; }
    float speed=distance/12.0f;
    speed=speed<0.075f?0.075f:(speed>0.11f?0.11f:speed);
    speed*=1.5f;
    float scale=speed/distance;
    out[0]=x*scale;out[1]=0;out[2]=z*scale;out[3]=0;
}

/* Native 2b6c03c supplies XMM0=0, RDI=cache, RSI=position. The first
 * parameter is float, not integer: an integer shifts both pointer arguments. */
EXPORT ABI void deke_board_cache_guest(address base,void *actor,void *pose,
        unsigned generation) {
    if (*(unsigned*)((u8*)actor+0x238)==generation) return;
    ((u8(ABI*)(float,void*,void*))(base+0x2a8dbd0))
        (0.0f,(u8*)actor+0x1d0,(u8*)pose+0x30);
    *(unsigned*)((u8*)actor+0x238)=generation;
}

/* Runs at the existing native bank-release join, before native puck launch.
 * Reverse variants 7..10 replace velocity; normal/behind-net banks keep the
 * native result. Geometry and virtual-foot queries use NHL24 services.
 */
EXPORT ABI void deke_bank_release_guest(address base,void *owner,float *out) {
    void *actor=pointer(owner,0x58),*context=context_for(actor);
    if (!context) return;
    unsigned *variant=0;
    if (deke_ant_bind_guest(base,context,0xabcc521ace57de1cULL,(void**)&variant) ||
            (*variant<7 || *variant>10)) return;
    void *pose=pointer(actor,0x10);
    if (!pose) return;
    if (*variant>=9) {
        void *puck=*(void**)(base+0xbae0ea8);
        if (!puck || !pointer(puck,0x10)) return;
        deke_soft_drop_velocity(out,(float*)((u8*)pose+0x30),scalar(pose,0x84),
            (float*)((u8*)pointer(puck,0x10)+0x30));
        return;
    }
    unsigned generation=*(unsigned*)(base+0xba45a04);
    deke_board_cache_guest(base,actor,pose,generation);
    deke_reverse_board_velocity(out,scalar(pose,0x30)-scalar(actor,0x1f0),
        scalar(pose,0x38)-scalar(actor,0x1f8),scalar(pose,0x84),
        scalar(pose,0x90),scalar(actor,0x1d4));
}
