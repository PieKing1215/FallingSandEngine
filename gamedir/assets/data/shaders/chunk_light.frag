#version 140

in vec2 tex_c;
in vec2 world_pos;
out vec4 color;

uniform vec2 player_light_world_pos;
uniform bool smooth_lighting;
uniform int chunk_size;
uniform sampler2D tex;
uniform sampler2D light_tex;

float noise(vec2 c) {
   return fract(sin(dot(c, vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    vec2 coord = tex_c;
    if (!smooth_lighting) coord = floor(tex_c * chunk_size) / chunk_size;

    // could use bicubic (ie. https://stackoverflow.com/a/42179924) but not very noticeable
    vec3 v = texture(tex, coord + vec2(0.5 / chunk_size)).rgb;

    float dst_to_player = distance(world_pos, player_light_world_pos);
    float d = 1.0/(dst_to_player / 5.0 + 1.0) + dst_to_player / 5.0;
    float player_light = 1.0 / (d + 1.0);
    v += player_light * vec3(1.0, 0.9, 0.8);
    
    if (v.r > 1.0) v = v / v.r;
    if (v.g > 1.0) v = v / v.g;
    if (v.b > 1.0) v = v / v.b;

    color = vec4(vec3(v), 1.0);
    color += mix(-4.0/255.0, 4.0/255.0, noise(smooth_lighting ? floor(tex_c * (chunk_size * 4.0)) / (chunk_size * 4.0) : coord )) * (smooth_lighting ? 1.0 : 0.5);
}