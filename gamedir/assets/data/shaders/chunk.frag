#version 140

in vec2 tex_c;
out vec4 color;

uniform sampler2D tex;
uniform sampler2D light_tex;

void main() {
    vec4 tex_color = texture(tex, tex_c);
    if (tex_color.a < 0.01) {
        tex_color = vec4(1.0, 0.0, 0.0, 1.0);
    }
    vec4 light = texture(light_tex, tex_c);
    color = vec4(tex_color.rgb * light.r, tex_color.a);
    // color = light;
}