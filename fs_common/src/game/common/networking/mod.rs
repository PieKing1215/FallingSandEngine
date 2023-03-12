use super::world::material::{color::Color, MaterialInstance};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Packet {
    pub packet_type: PacketType,
}

#[derive(Serialize, Deserialize)]
pub struct PVec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize)]
pub enum PacketType {
    SyncChunkPacket {
        chunk_x: i32,
        chunk_y: i32,
        pixels: Vec<MaterialInstance>,
        colors: Vec<Color>,
    },
    SyncLiquidFunPacket {
        //TODO: this assumes the number of particles never changes
        //      should probably send the entire state of rapier/salva (or only nearby?)
        positions: Vec<PVec2>,
        velocities: Vec<PVec2>,
    },
}
