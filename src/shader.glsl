-- vertex
#version 330 core

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 i_center;
layout(location = 2) in float i_radius;

uniform mat4 ortho;

void main() {
    gl_Position = ortho * vec4(i_radius * position + i_center, 0.0, 1.0);
}

-- fragment
#version 330 core

out vec4 frag_color;
void main() {
    frag_color = vec4(1.0, 1.0, 0.5, 1.0);
}
