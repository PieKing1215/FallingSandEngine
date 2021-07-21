
use std::{io::{Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, ops::Add, time::{Duration, Instant}};

use liquidfun::box2d::common::math::Vec2;

use crate::game::{Game, common::{networking::{PVec2, Packet, PacketType}, world::{CHUNK_SIZE, Chunk, ChunkState}}};

use super::world::ServerChunk;

impl Game<ServerChunk> {
    #[profiling::function]
    pub fn run(&mut self) -> Result<(), String> {

        let net_listener = TcpListener::bind("127.0.0.1:6673").map_err(|e| e.to_string())?;
        net_listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let mut connections: Vec<(TcpStream, SocketAddr)> = Vec::new();

        let mut prev_tick_time = std::time::Instant::now();
        let mut prev_tick_lqf_time = std::time::Instant::now();

        let mut counter_last_frame = Instant::now();

        let mut do_tick_next = false;
        let mut do_tick_lqf_next = false;

        let mut ticks = 0;

        let mut lqf_ticks = 0;

        '_mainLoop: loop {
            
            if let Ok((mut stream, addr)) = net_listener.accept() {
                println!("[SERVER] Incoming Connection: {}", addr.to_string());
                stream.set_nonblocking(false).unwrap();
                if let Some(w) = &self.world {
                    for ci in &w.chunk_handler.loaded_chunks {
                        // println!("[SERVER] Writing SyncChunkPacket");
                        let (chunk_x, chunk_y) = w.chunk_handler.chunk_index_inv(*ci.0);
                        let packet = Packet{ 
                            packet_type: PacketType::SyncChunkPacket {
                                chunk_x,
                                chunk_y,
                                pixels: ci.1.get_pixels().unwrap().to_vec(),
                                colors: ci.1.get_colors().to_vec(),
                            },
                        };
                        // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                        // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                        let buf = bincode::serialize(&packet).unwrap();
                        let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();
                        stream.write_all(&size_buf).unwrap();
                        stream.flush().unwrap();
                        stream.write_all(&buf).unwrap();
                        stream.flush().unwrap();

                        // println!("[SERVER] Wrote SyncChunkPacket");
                    }
                }
                stream.set_nonblocking(true).unwrap();
                connections.push((stream, addr));
            }

            for c in &mut connections {
                let mut buf = [0; 4];
                if let Ok(_) = c.0.read_exact(&mut buf) {
                    let size: u32 = bincode::deserialize(&buf).unwrap();
                    println!("[SERVER] Incoming packet, size = {}.", size);

                    let mut buf = Vec::with_capacity(size as usize);

                    println!("[SERVER] read_to_end...");
                    match std::io::Read::by_ref(&mut c.0).take(size as u64).read_to_end(&mut buf) {
                        Ok(_) => {
                            println!("[SERVER] Read {} bytes.", buf.len());
                            let p: Packet = bincode::deserialize(&buf).expect("[SERVER] Failed to deserialize packet.");
                            println!("[SERVER] Recieved packet from {:?}: {:?}", c.1, match p.packet_type {
                                PacketType::SyncChunkPacket{..} => "SyncChunkPacket",
                                _ => "???",
                            });
                        },
                        Err(e) => {
                            // TODO: this needs to be handled correctly like in client::game
                            //         since when read_to_end fails, it can still have read some of the bytes
                            panic!("[SERVER] read_to_end failed: {}", e);
                        },
                    }
                }
            }

            let now = std::time::Instant::now();

            // tick

            let can_tick = self.settings.tick;

            if do_tick_next && can_tick {
                if now.saturating_duration_since(prev_tick_time).as_millis() > 500 {
                    println!("[SERVER] 50+ ms behind, skipping some ticks to catch up...");
                    prev_tick_time = now;
                }else{
                    prev_tick_time = prev_tick_time.add(Duration::from_nanos(1_000_000_000 / self.settings.tick_speed as u64));
                }
                let st = Instant::now();
                self.tick();

                if self.tick_time % 4 == 0 {
                    if let Some(w) = &self.world {
                        let mut n = 0;
                        for ci in &w.chunk_handler.loaded_chunks {
                            n += 1;
                            if ci.1.get_state() == ChunkState::Active && ci.1.dirty {
                                if n % (self.tick_time / 4) % 4 == 0 {
                                    for c in &mut connections {
                                        // println!("[SERVER] Writing SyncChunkPacket");
                                        let (chunk_x, chunk_y) = w.chunk_handler.chunk_index_inv(*ci.0);
                                        let pixels_vec = ci.1.get_pixels().unwrap().to_vec();
                                        let colors_vec = ci.1.get_colors().to_vec();

                                        if pixels_vec.len() != (CHUNK_SIZE * CHUNK_SIZE) as usize {
                                            panic!("[SERVER] Almost sent wrong size pixels Vec: {} (expected {})", pixels_vec.len(), CHUNK_SIZE * CHUNK_SIZE);
                                        }
                                
                                        if colors_vec.len() != CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4 {
                                            panic!("[SERVER] Almost sent wrong size colors Vec: {} (expected {})", colors_vec.len(), CHUNK_SIZE as usize * CHUNK_SIZE as usize * 4);
                                        }

                                        let packet = Packet{ 
                                            packet_type: PacketType::SyncChunkPacket {
                                                chunk_x,
                                                chunk_y,
                                                pixels: pixels_vec,
                                                colors: colors_vec,
                                            },
                                        };
                                        // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                                        // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                                        let buf = bincode::serialize(&packet).unwrap();
                                        let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();

                                        c.0.set_nonblocking(false).unwrap();
                                        c.0.write_all(&size_buf).unwrap();
                                        c.0.flush().unwrap();
                                        c.0.write_all(&buf).unwrap();
                                        c.0.flush().unwrap();
                                        c.0.set_nonblocking(true).unwrap();
                
                                        // println!("[SERVER] Wrote SyncChunkPacket");
                                    }
                                }
                            }
                        }
                    }

                    // TODO: come up with a good way to merge this loop with the one right above
                    if let Some(w) = &mut self.world {
                        for ci in &mut w.chunk_handler.loaded_chunks {
                            if ci.1.get_state() == ChunkState::Active && ci.1.dirty {
                                ci.1.dirty = false;
                            }
                        }
                    }
                }

                self.fps_counter.tick_times.rotate_left(1);
                self.fps_counter.tick_times[self.fps_counter.tick_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;

                ticks += 1;
            }
            do_tick_next = can_tick && now.saturating_duration_since(prev_tick_time).as_nanos() > 1_000_000_000 / self.settings.tick_speed as u128; // intended is 30 ticks per second

            // tick liquidfun

            let can_tick = self.settings.tick_lqf;

            if do_tick_lqf_next && can_tick {
                if now.saturating_duration_since(prev_tick_lqf_time).as_millis() > 500 {
                    println!("[SERVER] liquidfun 50+ ms behind, skipping some ticks to catch up...");
                    prev_tick_lqf_time = now;
                }else{
                    prev_tick_lqf_time = prev_tick_lqf_time.add(Duration::from_nanos(1_000_000_000 / self.settings.tick_lqf_speed as u64));
                }
                if let Some(w) = &mut self.world {
                    let st = Instant::now();
                    w.tick_lqf(&self.settings);
                    lqf_ticks += 1;

                    if lqf_ticks % 10 == 0 {
                        let particle_system = w.lqf_world.get_particle_system_list().unwrap();

                        let particle_positions: &[Vec2] = particle_system.get_position_buffer();
                        let particle_velocities: &[Vec2] = particle_system.get_velocity_buffer();
                        for c in &mut connections {

                            let packet = Packet{ 
                                packet_type: PacketType::SyncLiquidFunPacket {
                                    positions: particle_positions.iter().map(|v2| PVec2 {x: v2.x, y: v2.y}).collect(),
                                    velocities: particle_velocities.iter().map(|v2| PVec2 {x: v2.x, y: v2.y}).collect(),
                                },
                            };
                            // let buf = serde_json::to_string(&packet).unwrap().into_bytes();
                            // let size_buf = serde_json::to_string(&(buf.len() as u32)).unwrap().into_bytes();
                            let buf = bincode::serialize(&packet).unwrap();
                            let size_buf = bincode::serialize(&(buf.len() as u32)).unwrap();

                            c.0.set_nonblocking(false).unwrap();
                            c.0.write_all(&size_buf).unwrap();
                            c.0.flush().unwrap();
                            c.0.write_all(&buf).unwrap();
                            c.0.flush().unwrap();
                            c.0.set_nonblocking(true).unwrap();

                            // println!("[SERVER] Wrote SyncChunkPacket");
                        }
                    }

                    self.fps_counter.tick_lqf_times.rotate_left(1);
                    self.fps_counter.tick_lqf_times[self.fps_counter.tick_lqf_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;
                    
                }
            }
            do_tick_lqf_next = can_tick && now.saturating_duration_since(prev_tick_lqf_time).as_nanos() > 1_000_000_000 / self.settings.tick_lqf_speed as u128; // intended is 60 ticks per second

            // render

            self.fps_counter.frames += 1;
            if now.saturating_duration_since(self.fps_counter.last_update).as_millis() >= 1000 {
                self.fps_counter.display_value = self.fps_counter.frames;
                self.fps_counter.frames = 0;
                self.fps_counter.last_update = now;
                
                let nums: Vec<f32> = self.fps_counter.frame_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspf: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                let nums: Vec<f32> = self.fps_counter.tick_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                let nums: Vec<f32> = self.fps_counter.tick_lqf_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_msplqft: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                println!("FPS: {}, TPS: {}, mspf: {:.2}, mspt: {:.2}, msplqft: {:.2}", self.fps_counter.display_value, ticks, avg_mspf, avg_mspt, avg_msplqft);
                ticks = 0;
            }

            let time_nano = Instant::now().saturating_duration_since(counter_last_frame).as_nanos();
            self.fps_counter.frame_times.rotate_left(1);
            self.fps_counter.frame_times[self.fps_counter.frame_times.len() - 1] = time_nano as f32;

            profiling::finish_frame!();
            // sleep
            if !do_tick_next {
                profiling::scope!("sleep");
                // let now = Instant::now();

                // TODO: this sleep is sleeping for like 15ms at a time on my system; figure out what the correct way to handle loop timing is
                ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
                
                // println!("slept {}ms", Instant::now().saturating_duration_since(now).as_millis());
            }
            counter_last_frame = Instant::now();
        }

        println!("Closing...");

        Ok(())
    }

    #[profiling::function]
    fn tick(&mut self){
        self.tick_time += 1;

        if let Some(w) = &mut self.world {
            w.tick(self.tick_time, &self.settings);
        }
    }
}