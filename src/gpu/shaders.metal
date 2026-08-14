#include <metal_stdlib>
using namespace metal;

struct VsOut {
    float4 position [[position]];
    float4 color;
};

struct Uniforms {
    float2 cell;
    float2 screen;
};

vertex VsOut vs_main(uint vid [[vertex_id]],
                     uint iid [[instance_id]],
                     constant float2* positions [[buffer(0)]],
                     constant float2* offsets [[buffer(1)]],
                     constant float4* colors [[buffer(2)]],
                     constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    float2 px = (offsets[iid] + positions[vid]) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;
    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);

    out.color = colors[iid];
    return out;
}

fragment float4 fs_main(VsOut in [[stage_in]]) {
    return in.color;
}
