use glium::Display;


pub struct Shaders {
    // pub liquid_shader: Shader,
    pub basic_shader: glium::Program,
    pub shader_vertex_colors: glium::Program,
}

impl Shaders {
    pub fn new(display: &Display) -> Self {

        let vertex_shader_src = r#"
            #version 140

            in vec2 position;

            out vec4 frag_col;

            uniform vec4 col;
            uniform mat4 matrix;

            void main() {
                frag_col = col;
                gl_Position = matrix * vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 140
            
            in vec4 frag_col;

            out vec4 color;

            void main() {
                color = frag_col;
            }
        "#;

        let basic_shader = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let vertex_shader_src = r#"
            #version 140

            in vec2 position;
            in vec4 color;

            out vec4 frag_col;

            uniform mat4 matrix;

            void main() {
                frag_col = color;
                gl_Position = matrix * vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 140
            
            in vec4 frag_col;

            out vec4 color;

            void main() {
                color = frag_col;
            }
        "#;

        let shader_vertex_colors = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        Self {
            basic_shader,
            shader_vertex_colors,
        }
    }
}