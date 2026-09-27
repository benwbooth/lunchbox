#version 450
// Run for one pixel in a tiny buffer, not once per output pixel. Find only complete black
// source-edge rows/columns; never infer padding from a dark individual pixel.
// This does not crop, scale, or move the game. It only identifies the empty
// inner lip where reflected light is safe to draw.
layout(std140, set = 0, binding = 0) uniform UBO { mat4 MVP; } global;
#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;
void main() {
    gl_Position = global.MVP * Position;
    vTexCoord = TexCoord;
}
#pragma stage fragment
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;
layout(set = 0, binding = 1) uniform sampler2D Original;

float lunchpailBlankEdge(ivec2 size, int axis, bool opposite) {
    int width = axis == 0 ? size.x : size.y;
    int height = axis == 0 ? size.y : size.x;
    int limit = int(float(width) * 0.08);
    for (int offset = 0; offset < limit; ++offset) {
        int along = opposite ? width - 1 - offset : offset;
        for (int cross = 0; cross < height; ++cross) {
            ivec2 p = axis == 0 ? ivec2(along, cross) : ivec2(cross, along);
            vec3 color = texelFetch(Original, p, 0).rgb;
            if (max(max(color.r, color.g), color.b) > 0.5 / 255.0)
                return float(offset) / float(width);
        }
    }
    // A dark scene or a large letterbox is not a narrow TV lip.
    return 0.0;
}

vec4 lunchpailFindPictureBounds() {
    ivec2 size = textureSize(Original, 0);
    // Bound the work for high-resolution cores. Their ordinary outside-screen
    // lip still reflects; do not scan thousands of rows for native padding.
    if (max(size.x, size.y) > 1024)
        return vec4(0.0, 0.0, 1.0, 1.0);
    return vec4(lunchpailBlankEdge(size, 0, false),
                lunchpailBlankEdge(size, 1, false),
                1.0 - lunchpailBlankEdge(size, 0, true),
                1.0 - lunchpailBlankEdge(size, 1, true));
}

void main() {
    // A 1x1 target is not reliably rasterized by every RetroArch GL driver.
    // Allocate 4x4, but perform the analysis only at the texel we read later.
    FragColor = vec4(0.0);
    if (ivec2(gl_FragCoord.xy) == ivec2(2))
        FragColor = lunchpailFindPictureBounds();
}
