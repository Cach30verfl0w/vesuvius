#version 450
#extension GL_ARB_separate_shader_objects : enable

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec4 outColor;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    outColor = vec4(color, 1.0);
}
