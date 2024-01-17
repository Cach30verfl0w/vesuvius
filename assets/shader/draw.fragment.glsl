#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(set = 0, binding = 0) uniform AlphaUniform {
    float alpha;
};

layout(location = 0) in vec4 inColor;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(inColor.xyz, alpha);
}
