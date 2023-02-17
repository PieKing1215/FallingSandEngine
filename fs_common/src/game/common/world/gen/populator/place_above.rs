use crate::game::common::{
    world::{
        material::{self, MaterialInstance},
        CHUNK_SIZE,
    },
    Registries,
};

use super::{ChunkContext, Populator};

pub struct PlaceAbovePopulator {
    /// Amount to extend above the found surface
    pub add_surface_height: u8,
    /// Amount to "bury" into the found surface
    pub replace_surface_depth: u8,
    pub replace: fn(&MaterialInstance, i64, i64, &Registries) -> Option<MaterialInstance>,
    pub searching_for: fn(&MaterialInstance) -> bool,
}

impl Populator<1> for PlaceAbovePopulator {
    #[profiling::function]
    fn populate(&self, chunks: &mut ChunkContext<1>, _seed: i32, registries: &Registries) {
        let cofs_x = i64::from(chunks.center_chunk().0) * i64::from(CHUNK_SIZE);
        let cofs_y = i64::from(chunks.center_chunk().1) * i64::from(CHUNK_SIZE);

        let replace_surface_depth = i32::from(self.replace_surface_depth);
        let add_surface_height = i32::from(self.add_surface_height);

        for x in 0..i32::from(CHUNK_SIZE) {
            for y in 0..i32::from(CHUNK_SIZE) {
                let m = chunks.get(x, y).unwrap();
                if (self.searching_for)(m)
                    && chunks.get(x, y - 1).unwrap().material_id == material::AIR
                {
                    for dy in -add_surface_height..replace_surface_depth {
                        let m2 = chunks.get(x, y + dy).unwrap();
                        if let Some(rep) = (self.replace)(
                            m2,
                            cofs_x + i64::from(x),
                            cofs_y + i64::from(y) + i64::from(dy),
                            registries,
                        ) {
                            chunks.set(x, y + dy, rep).unwrap();
                        }
                    }
                }
            }
        }
    }
}
