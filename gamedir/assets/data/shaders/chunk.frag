#version 140

in vec2 tex_c;
in vec2 world_pos;
out vec4 color;

uniform sampler2D tex;

void main() {
    vec4 tex_color = texture(tex, tex_c);
    color = tex_color;
}