#version 140

in vec2 position;
in vec2 tex_coord;
out vec2 tex_c;

uniform mat4 matrix;
uniform vec2 c_pos;

void main() {
	tex_c = tex_coord;
	gl_Position = matrix * vec4(position + c_pos, 0.0, 1.0);
}