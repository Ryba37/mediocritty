#include <metal_stdlib>
using namespace metal;

constant float2 positions[] = {
    float2( 0.0,  0.5),
    float2(-0.5, -0.5),
    float2( 0.5, -0.5),
};

vertex float4 vs_main(uint vid [[vertex_id]]) {
    return float4(positions[vid], 0.0, 1.0);
}

fragment float4 fs_main() {
    return float4(0.71, 0.94, 0.89, 1.0);
}
