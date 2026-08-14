// Temporal Anti-Aliasing (TAA)
//
// Uses a sub-pixel jitter offset (applied per-frame to the projection matrix)
// and blends the current frame with a history buffer. Neighborhood clamping
// prevents ghosting by constraining the history color to the AABB of the
// 3x3 neighborhood around the current pixel.
//
// This is a simplified TAA implementation that does not use motion vectors
// (the velocity buffer is not available in the postprocess pass). Temporal
// blending still smooths edges over frames, but fast-moving content may
// exhibit some ghosting. A full implementation would require velocity-aware
// reprojection.

layout(set = 2, binding = 0)
uniform texture2D t_history;

layout(set = 2, binding = 1)
uniform sampler s_history;

// Jitter offset for the current frame (in pixels)
layout(std140, set = 2, binding = 2)
uniform u_taa_locals {
    vec2 jitter_offset;
    float blend_factor;
};

vec4 aa_apply(
    texture2D tex, sampler smplr,
    texture2D depth_tex, sampler depth_smplr,
    vec2 fragCoord,
    vec2 resolution
) {
    ivec2 pix = ivec2(fragCoord);

    // Current frame color
    vec4 current = texelFetch(sampler2D(tex, smplr), pix, 0);

    // History (previous frame) color — sample at jittered position
    vec2 history_uv = (fragCoord - jitter_offset) / resolution;
    vec4 history = texture(sampler2D(t_history, s_history), history_uv);

    // Neighborhood clamping: build AABB from 3x3 neighborhood of current frame
    vec3 color_min = current.rgb;
    vec3 color_max = current.rgb;

    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            if (x == 0 && y == 0) continue;
            ivec2 npix = pix + ivec2(x, y);
            vec3 ncolor = texelFetch(sampler2D(tex, smplr), npix, 0).rgb;
            color_min = min(color_min, ncolor);
            color_max = max(color_max, ncolor);
        }
    }

    // Clamp history color to neighborhood AABB to reduce ghosting
    vec3 clamped_history = clamp(history.rgb, color_min, color_max);

    // Blend current with clamped history
    vec3 result = mix(clamped_history, current.rgb, blend_factor);

    return vec4(result, current.a);
}
