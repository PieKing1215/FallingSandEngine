#version 140

in vec2 tex_c;
out vec4 color;
in float frag_tex_layer;

uniform sampler2DArray tex;

void main() {
    color = texture(tex, vec3(tex_c, frag_tex_layer));
}