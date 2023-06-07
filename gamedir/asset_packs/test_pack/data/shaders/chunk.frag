#version 140

in vec2 tex_c;
in vec2 world_pos;
out vec4 color;

uniform sampler2D tex;
uniform sampler2D tex_bg;

void main() {
    vec4 tex_color = texture(tex, tex_c);
    vec4 bg_color = texture(tex_bg, tex_c);
    vec3 col = tex_color.rgb * tex_color.a + bg_color.rgb * vec3(0.67) * (1.0 - tex_color.a);
    float alpha = tex_color.a * tex_color.a + bg_color.a * (1.0 - tex_color.a);
    color = vec4(col, alpha);
}