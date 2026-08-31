#include <metal_stdlib>
using namespace metal;

struct VsOut {
    float4 position [[position]];
    float4 color;
    // pixel x within a single cell, wrapped so the pattern repeats even
    // across a wide-char rect that spans two columns
    float cell_x;
    // pixel y within this instance's own rect, 0 at the rect's top
    float local_y;
    float rect_height [[flat]];
    uint style [[flat]];
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

struct UnderlineInstance {
    float4 color;
    float2 offset;
    float2 size;
    uint style;
    uint pad0;
    uint pad1;
    uint pad2;
};

constant uint STYLE_STRAIGHT = 0u;
constant uint STYLE_UNDERCURL = 1u;
constant uint STYLE_DOTTED = 2u;
constant uint STYLE_DASHED = 3u;
constant float PI = 3.14159265;

vertex VsOut vs_underline(uint vid [[vertex_id]],
                          uint iid [[instance_id]],
                          constant float2* positions [[buffer(0)]],
                          constant UnderlineInstance* instances [[buffer(1)]],
                          constant Uniforms& u [[buffer(3)]]) {
    VsOut out;

    UnderlineInstance inst = instances[iid];
    float2 local = positions[vid];

    float2 px = (inst.offset + local * inst.size) * u.cell;
    float2 ndc = px / u.screen * 2 - 1;

    out.position = float4(ndc.x, -ndc.y, 0.0, 1.0);
    out.color = inst.color;
    out.style = inst.style;
    out.cell_x = fmod(local.x * inst.size.x * u.cell.x, u.cell.x);
    out.local_y = local.y * inst.size.y * u.cell.y;
    out.rect_height = inst.size.y * u.cell.y;
    return out;
}

// a squiggle: the wave's vertical center swings across the band once per
// cell, clamped so it never rides outside the band it was given
float undercurl_alpha(VsOut in, constant Uniforms& u) {
    float half_band = min(max(u.undercurl_amplitude, 1.0), in.rect_height * 0.5 - 0.5);
    float center = in.rect_height * 0.5
        + half_band * cos(in.cell_x / u.cell.x * 2.0 * PI);

    float dist = abs(in.local_y - center) - u.underline_thickness * 0.5;
    return clamp(1.0 - dist, 0.0, 1.0);
}

// round dots spaced along a single row inside the band, anti-aliased by
// distance to the dot center so they stay round at any thickness
float dotted_alpha(VsOut in, constant Uniforms& u) {
    float radius = max(u.underline_thickness * 0.5, 0.6);
    float period = max(radius * 4.0, 2.0);
    float dot_y = min(u.underline_thickness, in.rect_height - radius);

    float x = fmod(in.cell_x, period) - period * 0.5;
    float d = length(float2(x, in.local_y - dot_y));
    return clamp(radius + 0.5 - d, 0.0, 1.0);
}

// a solid line with a gap in the middle of the cell, so adjacent cells'
// dashes line up into one continuous dashed run
float dashed_alpha(VsOut in, constant Uniforms& u) {
    float half_dash = floor(u.cell.x / 4.0 + 0.5);
    bool in_gap = in.cell_x > half_dash - 1.0 && in.cell_x < u.cell.x - half_dash;
    return in_gap ? 0.0 : 1.0;
}

fragment float4 fs_underline(VsOut in [[stage_in]], constant Uniforms& u [[buffer(3)]]) {
    float alpha = 1.0;

    if (in.style == STYLE_UNDERCURL) {
        alpha = undercurl_alpha(in, u);
    } else if (in.style == STYLE_DOTTED) {
        alpha = dotted_alpha(in, u);
    } else if (in.style == STYLE_DASHED) {
        alpha = dashed_alpha(in, u);
    }

    return float4(in.color.rgb, in.color.a * alpha);
}
