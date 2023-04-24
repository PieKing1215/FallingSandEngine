use chunksystem::{ChunkKey, ChunkQuery};

use super::{
    material::{MaterialInstance, PhysicsType},
    pixel_to_chunk, pixel_to_chunk_pos, pixel_to_pos_in_chunk, Chunk,
};

pub trait FSChunkAccess {
    fn pixel(&self, world_x: i64, world_y: i64) -> Result<&MaterialInstance, String>;
    fn set_pixel(
        &mut self,
        world_x: i64,
        world_y: i64,
        mat: MaterialInstance,
    ) -> Result<(), String>;

    fn replace_pixel<F>(&mut self, world_x: i64, world_y: i64, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>;

    fn displace_pixel(&mut self, world_x: i64, world_y: i64, material: MaterialInstance) -> bool;

    fn chunk_at_dyn(&self, chunk_pos: ChunkKey) -> Option<&dyn Chunk>;
    fn chunk_at_mut_dyn(&mut self, chunk_pos: ChunkKey) -> Option<&mut dyn Chunk>;

    fn is_pixel_loaded(&self, world_x: i64, world_y: i64) -> bool;
}

impl<Q: ChunkQuery> FSChunkAccess for Q
where
    Q::D: Chunk,
{
    #[inline]
    fn pixel(&self, world_x: i64, world_y: i64) -> Result<&MaterialInstance, String> {
        let Some(ch) = self.chunk_at(pixel_to_chunk_pos(world_x, world_y)) else {
            return Err("Position is not loaded".into());
        };

        let local = pixel_to_pos_in_chunk(world_x, world_y);
        ch.pixel(local)
    }

    #[inline]
    fn set_pixel(
        &mut self,
        world_x: i64,
        world_y: i64,
        mat: MaterialInstance,
    ) -> Result<(), String> {
        let Some(ch) = self.chunk_at_mut(pixel_to_chunk_pos(world_x, world_y)) else {
            return Err("Position is not loaded".into());
        };

        let local = pixel_to_pos_in_chunk(world_x, world_y);

        ch.set_pixel(local, mat)
    }

    #[inline]
    fn replace_pixel<F>(&mut self, world_x: i64, world_y: i64, cb: F) -> Result<bool, String>
    where
        Self: Sized,
        F: FnOnce(&MaterialInstance) -> Option<MaterialInstance>,
    {
        let (chunk_pos, local) = pixel_to_chunk(world_x, world_y);
        let Some(ch) = self.chunk_at_mut(chunk_pos) else {
            return Err("Position is not loaded".into());
        };
        ch.replace_pixel(local, cb)
    }

    #[inline]
    fn chunk_at_dyn(&self, chunk_pos: ChunkKey) -> Option<&dyn Chunk> {
        self.chunk_at(chunk_pos).map(|ch| &ch.data as &dyn Chunk)
    }

    #[inline]
    fn chunk_at_mut_dyn(&mut self, chunk_pos: ChunkKey) -> Option<&mut dyn Chunk> {
        self.chunk_at_mut(chunk_pos)
            .map(|ch| &mut ch.data as &mut dyn Chunk)
    }

    #[inline]
    fn is_pixel_loaded(&self, world_x: i64, world_y: i64) -> bool {
        self.is_chunk_loaded(pixel_to_chunk_pos(world_x, world_y))
    }

    #[profiling::function]
    fn displace_pixel(&mut self, world_x: i64, world_y: i64, material: MaterialInstance) -> bool {
        let mut succeeded = false;

        let scan_w = 32;
        let scan_h = 32;
        let mut scan_x = 0;
        let mut scan_y = 0;
        let mut scan_delta_x = 0;
        let mut scan_delta_y = -1;
        let scan_max_i = scan_w.max(scan_h) * scan_w.max(scan_h); // the max is pointless now but could change w or h later

        for _ in 0..scan_max_i {
            if (scan_x >= -scan_w / 2)
                && (scan_x <= scan_w / 2)
                && (scan_y >= -scan_h / 2)
                && (scan_y <= scan_h / 2)
            {
                if let Ok(true) = self.replace_pixel(
                    world_x + i64::from(scan_x),
                    world_y + i64::from(scan_y),
                    |scan_mat| (scan_mat.physics == PhysicsType::Air).then_some(material.clone()),
                ) {
                    succeeded = true;
                    break;
                }
            }

            // update scan coordinates

            if (scan_x == scan_y)
                || ((scan_x < 0) && (scan_x == -scan_y))
                || ((scan_x > 0) && (scan_x == 1 - scan_y))
            {
                let temp = scan_delta_x;
                scan_delta_x = -scan_delta_y;
                scan_delta_y = temp;
            }

            scan_x += scan_delta_x;
            scan_y += scan_delta_y;
        }

        succeeded
    }
}
