use fs_common::game::common::world::chunk_data::CommonChunkData;
use fs_common::game::common::world::material::color::Color;
use fs_common::game::common::world::material::MaterialInstance;
use fs_common::game::common::world::mesh;
use fs_common::game::common::world::Chunk;
use fs_common::game::common::world::ChunkState;
use fs_common::game::common::world::RigidBodyState;
use fs_common::game::common::world::CHUNK_SIZE;
use fs_common::game::common::Rect;

pub struct ServerChunk {
    pub data: CommonChunkData,
    pub pixel_data: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    pub light_data: Box<[[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize]>,
    pub background_data: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    pub dirty: bool,
}

impl Chunk for ServerChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            data: CommonChunkData::new(chunk_x, chunk_y),
            pixel_data: Box::new([0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)]),
            light_data: Box::new([[0.0; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize]),
            background_data: Box::new([0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)]),
            dirty: true,
        }
    }

    fn chunk_x(&self) -> i32 {
        self.data.chunk_x
    }

    fn chunk_y(&self) -> i32 {
        self.data.chunk_y
    }

    fn state(&self) -> ChunkState {
        self.data.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.data.state = state;
    }

    fn dirty_rect(&self) -> Option<Rect<i32>> {
        self.data.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>) {
        self.data.dirty_rect = rect;
    }

    fn refresh(&mut self) {}

    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        self.data.set(x, y, mat, |_| Ok(()))
    }

    unsafe fn set_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        self.data.set_unchecked(x, y, mat)
    }

    fn pixel(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        self.data.pixel(x, y)
    }

    unsafe fn pixel_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        self.data.pixel_unchecked(x, y)
    }

    fn replace_pixel<F>(&mut self, x: u16, y: u16, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        self.data.replace_pixel(x, y, cb, |_| Ok(()))
    }

    fn set_light(&mut self, x: u16, y: u16, light: [f32; 3]) -> Result<(), String> {
        self.data.set_light(x, y, light, |_| Ok(()))
    }

    unsafe fn set_light_unchecked(&mut self, x: u16, y: u16, light: [f32; 3]) {
        self.data.set_light_unchecked(x, y, light)
    }

    fn light(&self, x: u16, y: u16) -> Result<&[f32; 3], String> {
        self.data.light(x, y)
    }

    unsafe fn light_unchecked(&self, x: u16, y: u16) -> &[f32; 3] {
        self.data.light_unchecked(x, y)
    }

    fn set_color(&mut self, x: u16, y: u16, color: Color) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            let i = (x + y * CHUNK_SIZE) as usize;

            self.pixel_data[i * 4] = color.r;
            self.pixel_data[i * 4 + 1] = color.g;
            self.pixel_data[i * 4 + 2] = color.b;
            self.pixel_data[i * 4 + 3] = color.a;
            self.dirty = true;

            return Ok(());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    fn color(&self, x: u16, y: u16) -> Result<Color, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            let i = (x + y * CHUNK_SIZE) as usize;

            return Ok(Color::rgba(
                self.pixel_data[i * 4],
                self.pixel_data[i * 4 + 1],
                self.pixel_data[i * 4 + 2],
                self.pixel_data[i * 4 + 3],
            ));
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>) {
        self.data.set_pixels(pixels);
    }

    fn pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &mut self.data.pixels
    }

    fn pixels(&self) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &self.data.pixels
    }

    fn set_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    ) {
        self.pixel_data = colors;
    }

    fn colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.pixel_data
    }

    fn colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.pixel_data
    }

    fn set_background_pixels(
        &mut self,
        pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>,
    ) {
        self.data.background = Some(pixels);
    }

    fn background_pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &mut self.data.background
    }

    fn background_pixels(
        &self,
    ) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &self.data.background
    }

    fn set_background_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    ) {
        self.background_data = colors;
    }

    fn background_colors_mut(
        &mut self,
    ) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.background_data
    }

    fn background_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.background_data
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn generate_mesh(&mut self) -> Result<(), String> {
        if self.data.pixels.is_none() {
            return Err("generate_mesh failed: self.data.pixels is None".to_owned());
        }

        let vs: Vec<f64> = mesh::pixels_to_valuemap(self.data.pixels.as_ref().unwrap().as_ref());

        let generated =
            mesh::generate_mesh_only_simplified(&vs, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));

        self.data.mesh_simplified = generated.ok();

        Ok(())
    }

    fn mesh_loops(&self) -> &Option<Vec<Vec<Vec<Vec<f64>>>>> {
        &self.data.mesh_simplified
    }

    fn rigidbody(&self) -> &Option<RigidBodyState> {
        &self.data.rigidbody
    }

    fn rigidbody_mut(&mut self) -> &mut Option<RigidBodyState> {
        &mut self.data.rigidbody
    }

    fn set_rigidbody(&mut self, body: Option<RigidBodyState>) {
        self.data.rigidbody = body;
    }

    fn lights_mut(&mut self) -> &mut [[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize] {
        self.light_data.as_mut()
    }

    fn lights(&self) -> &[[f32; 4]; CHUNK_SIZE as usize * CHUNK_SIZE as usize] {
        self.light_data.as_ref()
    }

    // #[profiling::function] // huge performance impact
    fn set_background(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        self.data.set_background(x, y, mat, |_| Ok(()))
    }

    unsafe fn set_background_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        self.data.set_background_unchecked(x, y, mat);
    }

    // #[profiling::function] // huge performance impact
    fn background(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        self.data.background(x, y)
    }

    unsafe fn background_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        self.data.background_unchecked(x, y)
    }
}
