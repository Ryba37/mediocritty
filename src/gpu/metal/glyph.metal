#include <metal_stdlib>
using namespace metal;

struct VsOut {
    float4 position [[position]];
    float4 color;
    float2 uv;
};

struct Uniforms {
    float2 cell;
    float2 screen;
    float2 atlas;
    uint cols;
    uint pad;
};

struct GlyphInstance {
    float4 color;
    float2 offset;
    uint cell;
    uint pad;
};

vertex VsOut vs_main(uint vid [[vertex_id]],
                     uint iid [[instance_id]],
                     constant float2* positions [[buffer(0)]],
                     constant GlyphInstance* instances [[buffer(1)]],
                     constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    GlyphInstance inst = instances[iid];

    float2 px = (inst.offset + positions[vid]) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    float2 origin = float2(inst.cell % u.cols, inst.cell / u.cols) * u.cell;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = (origin + positions[vid] * u.cell) / u.atlas;
    out.color = inst.color;
    return out;
}

fragment float4 fs_main(VsOut in [[stage_in]],
                        texture2d<float> atlas [[texture(0)]],
                        sampler samp [[sampler(0)]]) {
    float coverage = atlas.sample(samp, in.uv).r;
    return float4(in.color.rgb, in.color.a * coverage);
}
