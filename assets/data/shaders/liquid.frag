#version 120

in vec4 color;
in vec2 texCoord;

uniform sampler2D tex;

#define WIDTH 1920/2
#define HEIGHT 1080/2

float calc_weight(float dist) {
	if(dist > 1.0) return 0.0;
	if(dist > 0.3333) return 3.0/2.0 * (1 - dist) * (1 - dist);
	return 1.0 - 3 * dist * dist;
}

void main(void) {
	vec2 pixel_pos = floor(texCoord * vec2(WIDTH, HEIGHT));
	vec4 orig_col = texture2D(tex, texCoord);

	vec4 total_color = vec4(0.0, 0.0, 0.0, 0.0);
	float num_color = 0.0;

	float radius = 6.0;
	for(float x = -radius; x <= radius; x++) {
		for(float y = -radius; y <= radius; y++) {
			vec2 check_pos = pixel_pos + vec2(x, y) + vec2(0.5, 0.5);
			vec4 check_col = texture2D(tex, check_pos / vec2(WIDTH, HEIGHT));
			if(check_col.a > 0.01) {
				float dist = distance(pixel_pos, check_pos);
				float weight = clamp(calc_weight(dist / radius), 0.0, 1.0);
				total_color += check_col * weight;
				num_color += weight;
			}
		}
	}

	bool soft_edges = false;

	if(soft_edges && num_color > 0.4){
		vec4 col = total_color / num_color;
		
		float alpha = clamp((num_color - 0.4) / 0.2, 0.0, 1.0);
		float compression = clamp((num_color - 4.0) / 4.0, 0.0, 1.0);
		float darken = (1 - compression) * 0.1 + 0.9;

		gl_FragColor = vec4(col.rgb * darken, col.a * alpha);
	}else if(!soft_edges && num_color >= 0.9){
		vec4 col = total_color / num_color;

		float compression = clamp((num_color - 4.0) / 4.0, 0.0, 1.0);
		float darken = (1 - compression) * 0.1 + 0.9;

		gl_FragColor = vec4(col.rgb * darken, col.a);
	}else {
		gl_FragColor = vec4(0.0, 0.0, 0.0, 0.0);
	}

	// if(mod(pixel_pos.x, 2) >= 1) {
	// 	gl_FragColor = vec4(0.0, 1.0, 0.0, 0.5);
	// }else {
	// 	gl_FragColor = vec4(1.0, 0.0, 0.0, 0.5);
	// }
	// vec4 newCol = vec4(orig_col.r, orig_col.b, orig_col.g, orig_col.a);
	// gl_FragColor = newCol * color;
}