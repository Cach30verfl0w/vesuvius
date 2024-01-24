#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(set = 0, binding = 0) uniform sampler2D msdfSampler;

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 texCoord;

layout(location = 0) out vec4 outColor;

float screenPxRange() {
    // 8 = pxRange
    vec2 unitRange = vec2(8) / vec2(textureSize(msdfSampler, 0));
    vec2 screenTexSize = vec2(1.0) / fwidth(texCoord);
    return max(0.5 * dot(unitRange, screenTexSize), 1.0);
}

float median(float r, float g, float b) {
    return max(min(r, g), min(max(r, g), b));
}

void main() {
    // Temporary
    vec4 bgColor = vec4(0.0, 0.0, 0.0, 1.0);

    // Shader
    vec3 msd = texture(msdfSampler, texCoord).rgb;
    float screenPxDistance = screenPxRange() * (median(msd.r, msd.g, msd.b) - 0.5);
    float opacity = clamp(screenPxDistance + 0.5, 0.0, 1.0);
    outColor = mix(bgColor, inColor, opacity);
}