#include <metal_stdlib>
using namespace metal;

// GAMMA_BLEND: transparent window, non-sRGB target, so colors are encoded
// here and the fixed-function blend runs in gamma space like WindowServer
#ifdef GAMMA_BLEND
inline float3 out_rgb(float3 c) {
    return select(1.055 * pow(c, 1.0 / 2.4) - 0.055, c * 12.92, c <= 0.0031308);
}
#else
inline float3 out_rgb(float3 c) { return c; }
#endif

struct VsOut {
    float4 position [[position]];
    float4 color;
};

struct Uniforms {
    float2 cell;
    float2 screen;
    float2 atlas;
    uint cols;
    uint pad;
    float gamma;
    float contrast;
    float2 emoji_atlas;
    uint emoji_cols;
    uint emoji_pad;
    float underline_thickness;
    float undercurl_amplitude;
};

struct BgRect {
    float4 color;
    float2 offset;
    float2 size;
};

vertex VsOut vs_bg(uint vid [[vertex_id]],
                   uint iid [[instance_id]],
                   constant float2* positions [[buffer(0)]],
                   constant BgRect* rects [[buffer(1)]],
                   constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    BgRect r = rects[iid];

    float2 local = positions[vid] * r.size;
    float2 px = (r.offset + local) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.color = float4(out_rgb(r.color.rgb), r.color.a);
    return out;
}

fragment float4 fs_bg(VsOut in [[stage_in]]) {
    return in.color;
}
