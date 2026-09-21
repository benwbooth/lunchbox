#version 450

/* RetroTube aperture warp, ported from the RetroTube Shaders extension
   (presents the final picture with a rounded CRT aperture curvature and a
   feathered edge kill). Single pass, applied at viewport resolution. */

#pragma name retrotube_aperture_warp

layout(push_constant) uniform Push
{
    float warp;
} params;

#pragma parameter warp "Aperture warp" 0.1 0.0 0.18 0.005

layout(std140, set = 0, binding = 0) uniform UBO
{
    mat4 MVP;
    vec4 SourceSize;
    vec4 OutputSize;
} global;

#pragma stage vertex
layout(location = 0) in vec4 Position;
layout(location = 1) in vec2 TexCoord;
layout(location = 0) out vec2 vTexCoord;

void main()
{
    gl_Position = global.MVP * Position;
    vTexCoord = TexCoord;
}

#pragma stage fragment
layout(set = 0, binding = 2) uniform sampler2D Source;
layout(location = 0) in vec2 vTexCoord;
layout(location = 0) out vec4 FragColor;

void main()
{
    float feather = 1.5 / max(min(global.OutputSize.x, global.OutputSize.y), 1.0);
    vec2 centered = vTexCoord * 2.0 - 1.0;
    vec2 warped = centered * (1.0 + dot(centered, centered) * params.warp);
    vec2 uv = warped * 0.5 + 0.5;
    vec2 edge = min(uv, 1.0 - uv);
    float visible = smoothstep(-feather, feather, min(edge.x, edge.y));
    vec3 color = texture(Source, clamp(uv, 0.0, 1.0)).rgb;
    FragColor = vec4(color * visible, 1.0);
}
