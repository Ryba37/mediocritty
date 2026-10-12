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
    float2 uv;
    float gamma_mix;
    float contrast;
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

struct GlyphInstance {
    float4 color;
    float2 offset;
    uint cell;
    float gamma_mix;
};

constant uint WIDE_BIT = 1u << 31;
constant uint EXACT_BIT = 1u << 30;

vertex VsOut vs_main(uint vid [[vertex_id]],
                     uint iid [[instance_id]],
                     constant float2* positions [[buffer(0)]],
                     constant GlyphInstance* instances [[buffer(1)]],
                     constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    GlyphInstance inst = instances[iid];
    bool wide = (inst.cell & WIDE_BIT) != 0;
    bool exact = (inst.cell & EXACT_BIT) != 0;
    uint cell = inst.cell & ~(WIDE_BIT | EXACT_BIT);
    float2 scale = wide ? float2(2.0, 1.0) : float2(1.0, 1.0);

    float2 px = (inst.offset + positions[vid] * scale) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    float2 origin = float2(cell % u.cols, cell / u.cols) * u.cell;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = (origin + positions[vid] * u.cell * scale) / u.atlas;
    out.color = float4(out_rgb(inst.color.rgb), inst.color.a);
    out.gamma_mix = exact ? 0.0 : inst.gamma_mix * GAMMA_WEIGHT;
    out.contrast = exact ? 1.0 : u.contrast;
    return out;
}

fragment float4 fs_main(VsOut in [[stage_in]],
                        texture2d<float> atlas [[texture(0)]],
                        sampler samp [[sampler(0)]],
                        constant Uniforms& u [[buffer(3)]]) {
    float a = atlas.sample(samp, in.uv).r;
    a = saturate(mix(a, pow(a, u.gamma), in.gamma_mix) * in.contrast);
    return float4(in.color.rgb, in.color.a * a);
}
