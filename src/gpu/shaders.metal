#include <metal_stdlib>
using namespace metal;

vertex float4 vs_main(uint vid [[vertex_id]],
                      uint iid [[instance_id]],
                      constant float2* positions [[buffer(0)]],
                      constant float2* offsets [[buffer(1)]]) {
    return float4(positions[vid] + offsets[iid], 0.0, 1.0);
}

fragment float4 fs_main() {
    return float4(0.71, 0.94, 0.89, 1.0);
}
