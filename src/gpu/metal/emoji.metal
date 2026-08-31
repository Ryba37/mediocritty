#include <metal_stdlib>
using namespace metal;

struct VsOut {
    float4 position [[position]];
    float2 uv;
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

struct EmojiInstance {
    float2 offset;
    uint cell;
    uint pad;
};

constant uint WIDE_BIT = 1u << 31;

vertex VsOut vs_emoji(uint vid [[vertex_id]],
                      uint iid [[instance_id]],
                      constant float2* positions [[buffer(0)]],
                      constant EmojiInstance* instances [[buffer(1)]],
                      constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    EmojiInstance inst = instances[iid];
    bool wide = (inst.cell & WIDE_BIT) != 0;
    uint cell = inst.cell & ~WIDE_BIT;
    float2 scale = wide ? float2(2.0, 1.0) : float2(1.0, 1.0);

    float2 px = (inst.offset + positions[vid] * scale) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    // slot pitch is two cells wide, a narrow glyph samples the left half only
    float2 slot = float2(u.cell.x * 2.0, u.cell.y);
    float2 origin = float2(cell % u.emoji_cols, cell / u.emoji_cols) * slot;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.uv = (origin + positions[vid] * u.cell * scale) / u.emoji_atlas;
    return out;
}

fragment float4 fs_emoji(VsOut in [[stage_in]],
                         texture2d<float> atlas [[texture(0)]],
                         sampler samp [[sampler(0)]]) {
    // straight alpha, sRGB texture: the sampler linearizes rgb for us and the
    // pipeline's SourceAlpha blend does the multiply in linear space
    return atlas.sample(samp, in.uv);
}
