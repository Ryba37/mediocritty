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

vertex VsOut vs_main(uint vid [[vertex_id]],
                     uint iid [[instance_id]],
                     constant float2* positions [[buffer(0)]],
                     constant float2* offsets [[buffer(1)]],
                     constant float4* colors [[buffer(2)]],
                     constant Uniforms& u [[buffer(3)]],
                     constant uint* cells [[buffer(4)]]) {
    VsOut out;

    float2 px = (offsets[iid] + positions[vid]) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    uint n = cells[iid];
    float2 origin = float2(n % u.cols, n / u.cols) * u.cell;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = (origin + positions[vid] * u.cell) / u.atlas;
    out.color = colors[iid];
    return out;
}

fragment float4 fs_main(VsOut in [[stage_in]],
                        texture2d<float> atlas [[texture(0)]],
                        sampler samp [[sampler(0)]]) {
    float coverage = atlas.sample(samp, in.uv).r;
    return float4(in.color.rgb, in.color.a * coverage);
}
