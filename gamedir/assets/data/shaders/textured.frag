#version 140

in vec2 tex_c;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, tex_c);
}