#include <metal_stdlib>
using namespace metal;

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
    out.color = r.color;
    return out;
}

fragment float4 fs_bg(VsOut in [[stage_in]]) {
    return in.color;
}
