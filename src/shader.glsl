-- vertex
#version 330 core

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 i_center;
layout(location = 2) in float i_radius;

layout(location = 3) in vec3 i_color;

flat out vec3 color;

uniform mat4 ortho;

void main() {
    gl_Position = ortho * vec4(i_radius * position + i_center, 0.0, 1.0);
    color = i_color;
}

-- fragment
#version 330 core

out vec4 frag_color;
flat in vec3 color;

void main() {
    frag_color = vec4(color, 1.0);
}
