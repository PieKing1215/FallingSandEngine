use fs_common::game::common::world::chunk_data::CommonChunkData;
use fs_common::game::common::world::chunk_data::SidedChunkData;
use fs_common::game::common::world::material::color::Color;
use fs_common::game::common::world::material::MaterialInstance;
use fs_common::game::common::world::mesh;
use fs_common::game::common::world::tile_entity::TileEntity;
use fs_common::game::common::world::tile_entity::TileEntityCommon;
use fs_common::game::common::world::tile_entity::TileEntitySided;
use fs_common::game::common::world::Chunk;
use fs_common::game::common::world::ChunkLocalIndex;
use fs_common::game::common::world::ChunkLocalPosition;
use fs_common::game::common::world::ChunkRigidBodyState;
use fs_common::game::common::world::ChunkState;
use fs_common::game::common::world::SidedChunk;
use fs_common::game::common::world::CHUNK_AREA;
use fs_common::game::common::world::CHUNK_SIZE;
use fs_common::game::common::Rect;

pub struct ServerChunk {
    pub data: CommonChunkData<Self>,
    pub color_data: Box<[Color; CHUNK_AREA]>,
    pub light_data: Box<[[f32; 4]; CHUNK_AREA]>,
    pub background_data: Box<[Color; CHUNK_AREA]>,
    pub dirty: bool,
}

impl SidedChunkData for ServerChunk {
    type TileEntityData = TileEntityServer;
}

#[derive(Default)]
pub struct TileEntityServer;

impl TileEntitySided for TileEntityServer {
    type D = ServerChunk;
}

impl Chunk for ServerChunk {
    fn new_empty(chunk_x: i32, chunk_y: i32) -> Self {
        Self {
            data: CommonChunkData::new(chunk_x, chunk_y),
            color_data: Box::new([Color::TRANSPARENT; CHUNK_AREA]),
            light_data: Box::new([[0.0; 4]; CHUNK_AREA]),
            background_data: Box::new([Color::TRANSPARENT; CHUNK_AREA]),
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

    fn set_pixel(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) -> Result<(), String> {
        self.data.set(pos, mat, |_| Ok(()))
    }

    unsafe fn set_pixel_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) {
        self.data.set_unchecked(pos, mat)
    }

    fn pixel(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String> {
        self.data.pixel(pos)
    }

    unsafe fn pixel_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance {
        self.data.pixel_unchecked(pos)
    }

    fn replace_pixel<F>(&mut self, pos: ChunkLocalPosition, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        self.data.replace_pixel(pos, cb, |_| Ok(()))
    }

    fn set_light(&mut self, pos: ChunkLocalPosition, light: [f32; 3]) -> Result<(), String> {
        self.data.set_light(pos, light, |_| Ok(()))
    }

    unsafe fn set_light_unchecked(&mut self, pos: ChunkLocalPosition, light: [f32; 3]) {
        self.data.set_light_unchecked(pos, light)
    }

    fn light(&self, pos: ChunkLocalPosition) -> Result<&[f32; 3], String> {
        self.data.light(pos)
    }

    unsafe fn light_unchecked(&self, pos: ChunkLocalPosition) -> &[f32; 3] {
        self.data.light_unchecked(pos)
    }

    fn set_color(&mut self, pos: ChunkLocalPosition, color: Color) {
        let i: ChunkLocalIndex = pos.into();

        self.color_data[i] = color;
        self.dirty = true;
    }

    fn color(&self, pos: ChunkLocalPosition) -> Color {
        let i: ChunkLocalIndex = pos.into();
        self.color_data[i]
    }

    fn set_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>) {
        self.data.set_pixels(pixels);
    }

    fn pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &mut self.data.pixels
    }

    fn pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &self.data.pixels
    }

    fn set_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.color_data = colors;
    }

    fn colors_mut(&mut self) -> &mut [Color; CHUNK_AREA] {
        &mut self.color_data
    }

    fn colors(&self) -> &[Color; CHUNK_AREA] {
        &self.color_data
    }

    fn set_background_pixels(&mut self, pixels: Box<[MaterialInstance; CHUNK_AREA]>) {
        self.data.background = Some(pixels);
    }

    fn background_pixels_mut(&mut self) -> &mut Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &mut self.data.background
    }

    fn background_pixels(&self) -> &Option<Box<[MaterialInstance; CHUNK_AREA]>> {
        &self.data.background
    }

    fn set_background_pixel_colors(&mut self, colors: Box<[Color; CHUNK_AREA]>) {
        self.background_data = colors;
    }

    fn background_colors_mut(&mut self) -> &mut [Color; CHUNK_AREA] {
        &mut self.background_data
    }

    fn background_colors(&self) -> &[Color; CHUNK_AREA] {
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

    fn rigidbody(&self) -> &Option<ChunkRigidBodyState> {
        &self.data.rigidbody
    }

    fn rigidbody_mut(&mut self) -> &mut Option<ChunkRigidBodyState> {
        &mut self.data.rigidbody
    }

    fn set_rigidbody(&mut self, body: Option<ChunkRigidBodyState>) {
        self.data.rigidbody = body;
    }

    fn lights_mut(&mut self) -> &mut [[f32; 4]; CHUNK_AREA] {
        self.light_data.as_mut()
    }

    fn lights(&self) -> &[[f32; 4]; CHUNK_AREA] {
        self.light_data.as_ref()
    }

    // #[profiling::function] // huge performance impact
    fn set_background(
        &mut self,
        pos: ChunkLocalPosition,
        mat: MaterialInstance,
    ) -> Result<(), String> {
        self.data.set_background(pos, mat, |_| Ok(()))
    }

    unsafe fn set_background_unchecked(&mut self, pos: ChunkLocalPosition, mat: MaterialInstance) {
        self.data.set_background_unchecked(pos, mat);
    }

    // #[profiling::function] // huge performance impact
    fn background(&self, pos: ChunkLocalPosition) -> Result<&MaterialInstance, String> {
        self.data.background(pos)
    }

    unsafe fn background_unchecked(&self, pos: ChunkLocalPosition) -> &MaterialInstance {
        self.data.background_unchecked(pos)
    }

    fn add_tile_entity(&mut self, te: TileEntityCommon) {
        self.data.tile_entities.push(te.into());
    }

    fn common_tile_entities(&self) -> Box<dyn Iterator<Item = &TileEntityCommon> + '_> {
        Box::new(self.data.tile_entities.iter().map(|te| &te.common))
    }

    fn common_tile_entities_mut(&mut self) -> Box<dyn Iterator<Item = &mut TileEntityCommon> + '_> {
        Box::new(self.data.tile_entities.iter_mut().map(|te| &mut te.common))
    }
}

impl SidedChunk for ServerChunk {
    type S = Self;

    fn sided_tile_entities(&self) -> &[TileEntity<<Self::S as SidedChunkData>::TileEntityData>] {
        &self.data.tile_entities
    }

    fn sided_tile_entities_mut(
        &mut self,
    ) -> &mut [TileEntity<<Self::S as SidedChunkData>::TileEntityData>] {
        &mut self.data.tile_entities
    }

    fn sided_tile_entities_removable(
        &mut self,
    ) -> &mut Vec<TileEntity<<Self::S as SidedChunkData>::TileEntityData>> {
        &mut self.data.tile_entities
    }
}
