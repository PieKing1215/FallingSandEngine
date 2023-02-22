#version 140

in vec2 tex_c;
out vec4 color;

uniform sampler2D main_tiles;

void main() {
    vec4 main_color = texture(main_tiles, tex_c);
    vec2 light_pos = vec2(0.5, 0.5);
    // float dist = 1.0 - clamp(distance(tex_c, light_pos), 0.0, 1.0);
    // color = vec4(vec3(mod(dist * 10.0, 1.0)), 1.0);
    if (main_color.a < 0.01) {
        if (distance(tex_c, light_pos) < 0.1) {
            color = vec4(vec3(1.0), 1.0);
        } else {
            color = vec4(vec3(0.0), 1.0);
        }
    } else {
        color = vec4(vec3(1.0 - main_color.a), 1.0);
    }
}