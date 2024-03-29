#version 140

in vec2 position;

out vec4 frag_col;

uniform vec4 col;
uniform mat4 matrix;

void main() {
	frag_col = col;
	gl_Position = matrix * vec4(position, 0.0, 1.0);
}