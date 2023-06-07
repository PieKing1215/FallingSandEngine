use fs_common::game::common::FileHelper;
use glium::{
    program::{ComputeShader, ProgramChooserCreationError},
    Display, ProgramCreationError,
};

pub struct Shaders {
    pub common: glium::Program,
    pub vertex_colors: glium::Program,
    pub texture: glium::Program,
    pub texture_array: glium::Program,
    pub particle: glium::Program,
    pub chunk: glium::Program,
    pub chunk_light: glium::Program,
    pub lighting_compute_propagate: ComputeShader,
    pub lighting_compute_prep: ComputeShader,
}

impl Shaders {
    pub fn new(display: &Display, file_helper: &FileHelper) -> Self {
        profiling::scope!("Shaders::new");
        let helper = ShaderFileHelper { file_helper, display };

        Self {
            common: helper
                .load_from_files(140, "data/shaders/common.vert", "data/shaders/common.frag")
                .unwrap(),
            vertex_colors: helper
                .load_from_files(
                    140,
                    "data/shaders/vert_colors.vert",
                    "data/shaders/vert_colors.frag",
                )
                .unwrap(),
            texture: helper
                .load_from_files(
                    140,
                    "data/shaders/textured.vert",
                    "data/shaders/textured.frag",
                )
                .unwrap(),
            texture_array: helper
                .load_from_files(
                    140,
                    "data/shaders/texture_array.vert",
                    "data/shaders/texture_array.frag",
                )
                .unwrap(),
            particle: helper
                .load_from_files(
                    140,
                    "data/shaders/particles.vert",
                    "data/shaders/particles.frag",
                )
                .unwrap(),
            chunk: helper
                .load_from_files(140, "data/shaders/chunk.vert", "data/shaders/chunk.frag")
                .unwrap(),
            chunk_light: helper
                .load_from_files(
                    140,
                    "data/shaders/chunk.vert",
                    "data/shaders/chunk_light.frag",
                )
                .unwrap(),
            lighting_compute_propagate: helper
                .load_compute_from_files("data/shaders/lighting_propagate.comp")
                .unwrap(),
            lighting_compute_prep: helper
                .load_compute_from_files("data/shaders/lighting_prep.comp")
                .unwrap(),
        }
    }
}

pub struct ShaderFileHelper<'a> {
    pub file_helper: &'a FileHelper,
    pub display: &'a Display,
}

impl ShaderFileHelper<'_> {
    pub fn load_from_files(
        &self,
        version: u32,
        vert: &str,
        frag: &str,
    ) -> Result<glium::Program, ProgramChooserCreationError> {
        use glium::program;

        let vert = self
            .file_helper
            .read_asset_to_string(vert)
            .unwrap_or_else(|| panic!("Missing vertex shader {vert}"));
        let frag = self
            .file_helper
            .read_asset_to_string(frag)
            .unwrap_or_else(|| panic!("Missing fragment shader {frag}"));

        program!(self.display,
            version => {
                outputs_srgb: true,
                vertex: vert.as_str(),
                fragment: frag.as_str(),
            }
        )
    }

    #[profiling::function]
    pub fn load_compute_from_files(
        &self,
        src: &str,
    ) -> Result<glium::program::ComputeShader, ProgramCreationError> {
        let src = self
            .file_helper
            .read_asset_to_string(src)
            .unwrap_or_else(|| panic!("Missing compute shader {src}"));

        ComputeShader::from_source(self.display, &src)
    }
}
