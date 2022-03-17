use glium::{Display, program::ProgramCreationInput};


pub struct Shaders {
    // pub liquid_shader: Shader,
    pub basic_shader: glium::Program,
    pub shader_vertex_colors: glium::Program,
    pub texture: glium::Program,
    pub texture_array: glium::Program,
    pub particle: glium::Program,
    pub chunk: glium::Program,
}

impl Shaders {
    pub fn new(display: &Display) -> Self {

        // TODO: disable gamma correction on all of these

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

        let vertex_shader_src = r#"
            #version 140

            in vec2 position;
            in vec2 tex_coord;
            out vec2 tex_c;

            uniform mat4 matrix;

            void main() {
                tex_c = tex_coord;
                gl_Position = matrix * vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec2 tex_c;
            out vec4 color;

            uniform sampler2D tex;

            void main() {
                color = texture(tex, tex_c);
            }
        "#;

        let texture = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let vertex_shader_src = r#"
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
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec2 tex_c;
            out vec4 color;
            in float frag_tex_layer;

            uniform sampler2DArray tex;

            void main() {
                color = texture(tex, vec3(tex_c, frag_tex_layer));
            }
        "#;

        let texture_array = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let vertex_shader_src = r#"
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
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec4 frag_col;
            out vec4 color;

            void main() {
                color = frag_col;
            }
        "#;

        let particle = glium::Program::new(display, ProgramCreationInput::SourceCode {
            vertex_shader: vertex_shader_src,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            geometry_shader: None,
            fragment_shader: fragment_shader_src,
            transform_feedback_varyings: None,
            outputs_srgb: true,
            uses_point_size: false,
        }).unwrap();
        // let particle = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        let vertex_shader_src = r#"
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
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec2 tex_c;
            out vec4 color;

            uniform sampler2D tex;

            void main() {
                color = texture(tex, tex_c);
            }
        "#;

        let chunk = glium::Program::from_source(display, vertex_shader_src, fragment_shader_src, None).unwrap();

        Self {
            basic_shader,
            shader_vertex_colors,
            texture,
            texture_array,
            particle,
            chunk,
        }
    }
}