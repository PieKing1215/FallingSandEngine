use fs_common::game::common::world::material::color::Color;
use fs_common::game::common::world::material::MaterialInstance;
use fs_common::game::common::world::mesh;
use fs_common::game::common::world::Chunk;
use fs_common::game::common::world::ChunkState;
use fs_common::game::common::world::RigidBodyState;
use fs_common::game::common::world::CHUNK_SIZE;
use fs_common::game::common::Rect;

pub struct ServerChunk {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub state: ChunkState,
    pub pixels: Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>>,
    pub dirty_rect: Option<Rect<i32>>,
    pub pixel_data: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    pub dirty: bool,
    pub rigidbody: Option<RigidBodyState>,
    pub mesh_simplified: Option<Vec<Vec<Vec<Vec<f64>>>>>,
}

impl Chunk for ServerChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            chunk_x,
            chunk_y,
            state: ChunkState::NotGenerated,
            pixels: None,
            dirty_rect: None,
            pixel_data: Box::new([0; (CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4)]),
            dirty: true,
            rigidbody: None,
            mesh_simplified: None,
        }
    }

    fn get_chunk_x(&self) -> i32 {
        self.chunk_x
    }

    fn get_chunk_y(&self) -> i32 {
        self.chunk_y
    }

    fn get_state(&self) -> ChunkState {
        self.state
    }

    fn set_state(&mut self, state: ChunkState) {
        self.state = state;
    }

    fn get_dirty_rect(&self) -> Option<Rect<i32>> {
        self.dirty_rect
    }

    fn set_dirty_rect(&mut self, rect: Option<Rect<i32>>) {
        self.dirty_rect = rect;
    }

    fn refresh(&mut self) {}

    // #[profiling::function]
    fn update_graphics(&mut self) -> Result<(), String> {
        Ok(())
    }

    // #[profiling::function] // huge performance impact
    fn set(&mut self, x: u16, y: u16, mat: MaterialInstance) -> Result<(), String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                *unsafe { px.get_unchecked_mut(i) } = mat;

                self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                return Ok(());
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn set_unchecked(&mut self, x: u16, y: u16, mat: MaterialInstance) {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        *unsafe { self.pixels.as_mut().unwrap().get_unchecked_mut(i) } = mat;

        self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));
    }

    // #[profiling::function] // huge performance impact
    fn get(&self, x: u16, y: u16) -> Result<&MaterialInstance, String> {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                return Ok(unsafe { px.get_unchecked(i) });
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
    }

    unsafe fn get_unchecked(&self, x: u16, y: u16) -> &MaterialInstance {
        let i = (x + y * CHUNK_SIZE) as usize;
        // Safety: input index assumed to be valid
        unsafe { self.pixels.as_ref().unwrap().get_unchecked(i) }
    }

    fn replace<F>(&mut self, x: u16, y: u16, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        if x < CHUNK_SIZE && y < CHUNK_SIZE {
            if let Some(px) = &mut self.pixels {
                let i = (x + y * CHUNK_SIZE) as usize;
                // Safety: we do our own bounds check
                let px = unsafe { px.get_unchecked_mut(i) };
                if let Some(mat) = (cb)(px) {
                    *px = mat;

                    self.dirty_rect = Some(Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE));

                    return Ok(true);
                }

                return Ok(false);
            }

            return Err("Chunk is not ready yet.".to_string());
        }

        Err("Invalid pixel coordinate.".to_string())
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

    fn get_color(&self, x: u16, y: u16) -> Result<Color, String> {
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

    #[profiling::function]
    fn apply_diff(&mut self, diff: &[(u16, u16, MaterialInstance)]) {
        for (x, y, mat) in diff.iter() {
            self.set(*x, *y, mat.clone()).unwrap(); // TODO: handle this Err
        }
    }

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>) {
        self.pixels = Some(pixels);
    }

    fn get_pixels_mut(
        &mut self,
    ) -> &mut Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &mut self.pixels
    }

    fn get_pixels(&self) -> &Option<Box<[MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize]>> {
        &self.pixels
    }

    fn set_pixel_colors(
        &mut self,
        colors: Box<[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4]>,
    ) {
        self.pixel_data = colors;
    }

    fn get_colors_mut(&mut self) -> &mut [u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &mut self.pixel_data
    }

    fn get_colors(&self) -> &[u8; CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4] {
        &self.pixel_data
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn generate_mesh(&mut self) -> Result<(), String> {
        if self.pixels.is_none() {
            return Err("generate_mesh failed: self.pixels is None".to_owned());
        }

        let vs: Vec<f64> = mesh::pixels_to_valuemap(self.pixels.as_ref().unwrap().as_ref());

        let generated =
            mesh::generate_mesh_only_simplified(&vs, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));

        self.mesh_simplified = generated.ok();

        Ok(())
    }

    fn get_mesh_loops(&self) -> &Option<Vec<Vec<Vec<Vec<f64>>>>> {
        &self.mesh_simplified
    }

    fn get_rigidbody(&self) -> &Option<RigidBodyState> {
        &self.rigidbody
    }

    fn get_rigidbody_mut(&mut self) -> &mut Option<RigidBodyState> {
        &mut self.rigidbody
    }

    fn set_rigidbody(&mut self, body: Option<RigidBodyState>) {
        self.rigidbody = body;
    }
}
