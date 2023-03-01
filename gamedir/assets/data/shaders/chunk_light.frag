#version 140

in vec2 tex_c;
in vec2 world_pos;
out vec4 color;

uniform vec2 player_light_world_pos;
uniform bool smooth_lighting;
uniform int chunk_size;
uniform sampler2D tex;
uniform sampler2D light_tex;

void main() {
    vec2 coord = tex_c;
    if (!smooth_lighting) coord = floor(tex_c * chunk_size) / chunk_size;
    vec3 v = texture(tex, coord + vec2(0.5 / chunk_size)).rgb;

    float dst_to_player = distance(world_pos, player_light_world_pos);
    float d = 1.0/(dst_to_player / 5.0 + 1.0) + dst_to_player / 5.0;
    float player_light = 1.0 / (d + 1.0);
    v += vec3(player_light);

    color = vec4(vec3(v), 1.0);
}