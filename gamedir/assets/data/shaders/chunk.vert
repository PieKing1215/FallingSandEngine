#version 140

in vec2 position;
in vec2 tex_coord;
out vec2 tex_c;
out vec2 world_pos;

uniform mat4 matrix;
uniform vec2 c_pos;

void main() {
	tex_c = tex_coord;
	world_pos = position + c_pos;
	gl_Position = matrix * vec4(world_pos, 0.0, 1.0);
}