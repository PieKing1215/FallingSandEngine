pub mod biome;
pub mod biome_test;
pub mod feature;
pub mod populator;
pub mod structure;
mod test;

use std::boxed::Box;
use std::collections::HashMap;
use std::usize;
use std::{any::Any, vec::Vec};

pub use test::*;

use crate::game::common::world::gen::populator::ChunkContext;
use crate::game::common::world::Chunk;
use crate::game::Registries;

use self::feature::PlacedFeature;
use self::populator::Populator;

use super::{material::MaterialInstance, CHUNK_SIZE};

#[derive(Debug)]
pub struct PopulatorList {
    /// Invariant: for a given key S, the value must be `Box<Vec<Box<dyn Populator<S> + Send + Sync>>>`
    ///
    /// Afaik this is impossible to express statically
    map: HashMap<u8, Box<dyn Any + Send + Sync>>,
}

impl PopulatorList {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn add<const S: u8>(&mut self, pop: impl Populator<S> + 'static + Send + Sync) {
        let opt: Option<&mut Vec<Box<dyn Populator<S> + Send + Sync>>> = self
            .map
            .entry(S)
            .or_insert_with(|| Box::<Vec<Box<dyn Populator<S> + Send + Sync>>>::default())
            .downcast_mut();

        // Safety: this function is the only place where we insert into self.map, so the downcast cannot fail
        let vec = unsafe { opt.unwrap_unchecked() };

        vec.push(Box::new(pop));
    }

    pub fn get_all<const S: u8>(&self) -> &[Box<dyn Populator<S> + Send + Sync>] {
        if let Some(a) = self.map.get(&S) {
            // Safety: this is an invariant of self.map
            let vec: &Vec<Box<dyn Populator<S> + Send + Sync>> =
                unsafe { a.downcast_ref().unwrap_unchecked() };
            vec
        } else {
            &[]
        }
    }

    pub fn populate(
        &self,
        phase: u8,
        chunks: &mut [&mut dyn Chunk],
        seed: i32,
        registries: &Registries,
    ) {
        // convert from runtime variable to compile time const generics
        // not really sure if there's a better way to do this
        akin::akin! {
            let &lhs = [0, 1, 2, 3, 4, 5, 6, 7];
            let &branch = {
                *lhs => {
                    let mut ctx = ChunkContext::<*lhs>::new(chunks).unwrap();
                    for pop in self.get_all::<*lhs>() {
                        pop.populate(&mut ctx, seed, registries);
                    }
                }
            };
            match phase {
                *branch
                _ => {}
            }
        }
    }
}

// impl<const S: u8> Populator<S> for PopulatorList {
//     fn populate(&self, chunks: &mut populator::ChunkContext<S>, seed: i32, registries: &Registries) {
//         for pop in self.get_all::<S>() {
//             pop.populate(chunks, seed, registries);
//         }
//     }
// }

pub trait WorldGenerator: Send + Sync + std::fmt::Debug {
    #[allow(clippy::cast_lossless)]
    fn generate(
        &self,
        chunk_x: i32,
        chunk_y: i32,
        seed: i32,
        pixels: &mut [MaterialInstance; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        colors: &mut [u8; (CHUNK_SIZE as u32 * CHUNK_SIZE as u32 * 4) as usize],
        registries: &Registries,
    );

    fn max_gen_stage(&self) -> u8;

    fn populators(&self) -> &PopulatorList;

    fn features(&self) -> &[PlacedFeature];
}
