#version 140

in vec2 position;
in vec2 p_pos;
in vec4 color;

out vec4 frag_col;

uniform mat4 matrix;

void main() {
	frag_col = color;
	gl_Position = matrix * vec4(position + p_pos, 0.0, 1.0);
}