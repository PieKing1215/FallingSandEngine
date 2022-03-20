#version 140

in vec2 position;
in vec2 tex_coord;

// instance
in vec2 c_pos;
in float tex_layer;

out vec2 tex_c;
out float frag_tex_layer;

uniform mat4 matrix;

void main() {
	tex_c = tex_coord;
	frag_tex_layer = tex_layer; 
	gl_Position = matrix * vec4(position + c_pos, 0.0, 1.0);
}