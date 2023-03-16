pub mod biome;
pub mod biome_test;
pub mod feature;
pub mod populator;
pub mod structure;
mod test;

use std::boxed::Box;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::{any::Any, vec::Vec};

use chunksystem::ChunkKey;
pub use test::*;

use crate::game::common::world::gen::populator::ChunkContext;
use crate::game::common::world::Chunk;
use crate::game::common::Registries;

use self::feature::PlacedFeature;
use self::populator::Populator;

use super::material::color::Color;
use super::material::MaterialInstance;
use super::CHUNK_AREA;

#[derive(Debug)]
pub struct PopulatorList<C: Chunk> {
    /// Invariant: for a given key S, the value must be `Box<Vec<Box<dyn Populator<S, C> + Send + Sync>>>`
    ///
    /// Afaik this is impossible to express statically
    map: HashMap<u8, Box<dyn Any + Send + Sync>>,
    phantom: PhantomData<Box<dyn Populator<0, C> + Send + Sync>>,
}

impl<C: Chunk + 'static> PopulatorList<C> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { map: HashMap::new(), phantom: PhantomData }
    }

    pub fn add<const S: u8>(&mut self, pop: impl Populator<S, C> + 'static + Send + Sync) {
        let opt: Option<&mut Vec<Box<dyn Populator<S, C> + Send + Sync>>> = self
            .map
            .entry(S)
            .or_insert_with(|| Box::<Vec<Box<dyn Populator<S, C> + Send + Sync>>>::default())
            .downcast_mut();

        // Safety: this function is the only place where we insert into self.map, so the downcast cannot fail
        let vec = unsafe { opt.unwrap_unchecked() };

        vec.push(Box::new(pop));
    }

    pub fn get_all<const S: u8>(&self) -> &[Box<dyn Populator<S, C> + Send + Sync>] {
        if let Some(a) = self.map.get(&S) {
            // Safety: this is an invariant of self.map
            let vec: &Vec<Box<dyn Populator<S, C> + Send + Sync>> =
                unsafe { a.downcast_ref().unwrap_unchecked() };
            vec
        } else {
            &[]
        }
    }

    pub fn populate(&self, phase: u8, chunks: &mut [&mut C], seed: i32, registries: &Registries) {
        // convert from runtime variable to compile time const generics
        // not really sure if there's a better way to do this
        akin::akin! {
            let &lhs = [0, 1, 2, 3, 4, 5, 6, 7];
            let &branch = {
                *lhs => {
                    let mut ctx = ChunkContext::<*lhs, C>::new(chunks).unwrap();
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

pub trait WorldGenerator<C: Chunk>: Send + Sync {
    #[allow(clippy::cast_lossless)]
    #[warn(clippy::too_many_arguments)] // TODO
    fn generate(
        &self,
        chunk_pos: ChunkKey,
        seed: i32,
        pixels: &mut [MaterialInstance; CHUNK_AREA],
        colors: &mut [Color; CHUNK_AREA],
        background: &mut [MaterialInstance; CHUNK_AREA],
        background_colors: &mut [Color; CHUNK_AREA],
        registries: &Registries,
    );

    fn max_gen_stage(&self) -> u8;

    fn populators(&self) -> &PopulatorList<C>;

    fn features(&self) -> &[PlacedFeature<C>];
}
