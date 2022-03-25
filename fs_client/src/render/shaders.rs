use std::fs;

use fs_common::game::common::FileHelper;
use glium::{program::ProgramChooserCreationError, Display};

pub struct Shaders {
    pub common: glium::Program,
    pub vertex_colors: glium::Program,
    pub texture: glium::Program,
    pub texture_array: glium::Program,
    pub particle: glium::Program,
    pub chunk: glium::Program,
}

impl Shaders {
    pub fn new(display: &Display, file_helper: &FileHelper) -> Self {
        let helper = Helper { file_helper, display };

        Self {
            common: helper
                .from_files(140, "data/shaders/common.vert", "data/shaders/common.frag")
                .unwrap(),
            vertex_colors: helper
                .from_files(
                    140,
                    "data/shaders/vert_colors.vert",
                    "data/shaders/vert_colors.frag",
                )
                .unwrap(),
            texture: helper
                .from_files(
                    140,
                    "data/shaders/textured.vert",
                    "data/shaders/textured.frag",
                )
                .unwrap(),
            texture_array: helper
                .from_files(
                    140,
                    "data/shaders/texture_array.vert",
                    "data/shaders/texture_array.frag",
                )
                .unwrap(),
            particle: helper
                .from_files(
                    140,
                    "data/shaders/particles.vert",
                    "data/shaders/particles.frag",
                )
                .unwrap(),
            chunk: helper
                .from_files(140, "data/shaders/chunk.vert", "data/shaders/chunk.frag")
                .unwrap(),
        }
    }
}

struct Helper<'a> {
    file_helper: &'a FileHelper,
    display: &'a Display,
}

impl Helper<'_> {
    fn from_files(
        &self,
        version: u32,
        vert: &str,
        frag: &str,
    ) -> Result<glium::Program, ProgramChooserCreationError> {
        use glium::program;

        let vert = fs::read_to_string(self.file_helper.asset_path(vert)).unwrap();
        let frag = fs::read_to_string(self.file_helper.asset_path(frag)).unwrap();

        program!(self.display,
            version => {
                outputs_srgb: true,
                vertex: vert.as_str(),
                fragment: frag.as_str(),
            }
        )
    }
}
