
use std::{borrow::BorrowMut, collections::HashMap};

use crate::game::common::Settings;
use liquidfun::box2d::{collision::shapes::chain_shape::ChainShape, common::{b2draw, math::Vec2}, dynamics::body::{BodyDef, BodyType}, particle::{ParticleDef, TENSILE_PARTICLE, particle_system::ParticleSystemDef}};
use sdl2::pixels::Color;

use super::{CHUNK_SIZE, Chunk, ChunkHandler, entity::Entity, gen::{TEST_GENERATOR, TestGenerator}, material::{AIR, MaterialInstance, PhysicsType, TEST_MATERIAL}, rigidbody::RigidBody, simulator};

pub const LIQUIDFUN_SCALE: f32 = 10.0;

#[derive(Debug)]
pub enum WorldNetworkMode {
    Local,
    Remote,
}

pub struct World<C: Chunk> {
    pub chunk_handler: ChunkHandler<TestGenerator, C>,
    pub lqf_world: liquidfun::box2d::dynamics::world::World,
    pub entities: HashMap<u32, Entity>,
    pub net_mode: WorldNetworkMode,
    pub rigidbodies: Vec<RigidBody>,
}

impl<'w, C: Chunk> World<C> {
    #[profiling::function]
    pub fn create() -> Self {
        let gravity = liquidfun::box2d::common::math::Vec2::new(0.0, 3.0);
        let mut lqf_world = liquidfun::box2d::dynamics::world::World::new(&gravity);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(0.0, -26.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(46.0, 0.4);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(0.0, 0.4);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(12.0, 0.4);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(12.0, -6.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(0.4, 6.0);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(-12.0, -6.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(0.4, 6.0);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(7.0, -8.3);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(0.2, 8.0);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut body_def = BodyDef::default();
        // body_def.body_type = BodyType::DynamicBody;
        // body_def.position.set(-1.0, -2.0);
        // body_def.angular_velocity = 2.0;
        // body_def.linear_velocity = Vec2::new(0.0, -4.0);
        // let body = lqf_world.create_body(&body_def);
        // let mut dynamic_box = PolygonShape::new();
        // dynamic_box.set_as_box(1.0, 1.0);
        // let mut fixture_def = FixtureDef::new(&dynamic_box);
        // fixture_def.density = 1.5;
        // fixture_def.friction = 0.3;
        // body.create_fixture(&fixture_def);

        // let mut body_def = BodyDef::default();
        // body_def.body_type = BodyType::DynamicBody;
        // body_def.position.set(-10.0, -2.0);
        // body_def.angular_velocity = 2.0;
        // body_def.linear_velocity = Vec2::new(0.0, -4.0);
        // let body = lqf_world.create_body(&body_def);
        // let mut dynamic_box = PolygonShape::new();
        // dynamic_box.set_as_box(1.0, 1.0);
        // let mut fixture_def = FixtureDef::new(&dynamic_box);
        // fixture_def.density = 0.75;
        // fixture_def.friction = 0.3;
        // body.create_fixture(&fixture_def);

        // bottom section

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(0.0, 15.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box(24.0, 0.4);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(35.0, -5.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box_oriented(0.4, 24.0, &Vec2{x: 0.0, y: 0.0}, 0.5);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        // let mut ground_body_def = BodyDef::default();
	    // ground_body_def.position.set(-35.0, -5.0);
        // let ground_body = lqf_world.create_body(&ground_body_def);
        // let mut ground_box = PolygonShape::new();
        // ground_box.set_as_box_oriented(0.4, 24.0, &Vec2{x: 0.0, y: 0.0}, -0.5);
        // ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let particle_system_def = ParticleSystemDef {
            radius: 0.19, 
            surface_tension_pressure_strength: 0.1, 
            surface_tension_normal_strength: 0.1, 
            damping_strength: 0.001, 
            ..ParticleSystemDef::default() 
        };
	    let particle_system = lqf_world.create_particle_system(&particle_system_def);
        let mut pd = ParticleDef::default();
        pd.flags.insert(TENSILE_PARTICLE);
        pd.color.set(255, 90, 255, 255);

        // for i in 0..15000 {
        //     if i < 15000/2 {
        //         pd.color.set(255, 200, 64, 191);
        //     }else {
        //         pd.color.set(64, 200, 255, 191);
        //     }
        //     pd.position.set(-7.0 + (i as f32 / 200.0) * 0.17, -6.0 - ((i % 200) as f32) * 0.17);
        //     particle_system.create_particle(&pd);
        // }

        let mut w = World {
            chunk_handler: ChunkHandler::new(TEST_GENERATOR),
            lqf_world,
            entities: HashMap::new(),
            net_mode: WorldNetworkMode::Local,
            rigidbodies: Vec::new(),
        };

        // add a rigidbody

        let pixels = (0..40 * 40).map(|i| {
            let x: i32 = i % 40;
            let y: i32 = i / 40;
            if (x - 20).abs() < 5 || (y - 20).abs() < 5 {
                MaterialInstance {
                    material_id: TEST_MATERIAL.id,
                    physics: PhysicsType::Solid,
                    color: Color::RGB(64, if (x + y) % 4 >= 2 { 191 } else { 64 }, if (x + y) % 4 > 2 { 64 } else { 191 }),
                }
            }else {
                MaterialInstance::air()
            }
        }).collect();
        
        if let Ok(mut r) = RigidBody::make_bodies(pixels, 40, 40, &mut w.lqf_world, (-1.0, -7.0)) {
            w.rigidbodies.append(&mut r);
        }

        // add another rigidbody

        let pixels = (0..40 * 40).map(|i| {
            let x: i32 = i % 40;
            let y: i32 = i / 40;
            let dst = (x - 20) * (x - 20) + (y - 20) * (y - 20);
            if dst <= 10 * 10 {
                MaterialInstance {
                    material_id: TEST_MATERIAL.id,
                    physics: PhysicsType::Sand,
                    color: Color::RGB(255, 64, 255),
                }
            }else if dst <= 20 * 20 && ((x - 20).abs() >= 5 || y > 20) {
                MaterialInstance {
                    material_id: TEST_MATERIAL.id,
                    physics: PhysicsType::Solid,
                    color: Color::RGB(if (x + y) % 4 >= 2 { 191 } else { 64 }, if (x + y) % 4 > 2 { 64 } else { 191 }, 64),
                }
            }else {
                MaterialInstance::air()
            }
        }).collect();
        
        if let Ok(mut r) = RigidBody::make_bodies(pixels, 40, 40, &mut w.lqf_world, (2.0, -6.5)) {
            w.rigidbodies.append(&mut r);
        }

        for n in 0..4 {
            // add more rigidbodies

            let pixels = (0..30 * 30).map(|i| {
                let x: i32 = i % 30 + (((i + n * 22) as f32 / 60.0).sin() * 2.0) as i32;
                let y: i32 = i / 30;
                let dst = (x - 15) * (x - 15) + (y - 15) * (y - 15);
                if dst > 5 * 5 && dst <= 10 * 10  {
                    MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Solid,
                        color: Color::RGB(if (x + y) % 4 >= 2 { 191 } else { 64 }, if (x + y) % 4 > 2 { 64 } else { 191 }, if (x + y) % 4 >= 2 { 191 } else { 64 }),
                    }
                }else {
                    MaterialInstance::air()
                }
            }).collect();
            
            if let Ok(mut r) = RigidBody::make_bodies(pixels, 30, 30, &mut w.lqf_world, (5.0 + n as f32 * 2.0, -7.0 + n as f32 * -0.75)) {
                w.rigidbodies.append(&mut r);
            }
        }

        w
    }

    pub fn add_entity(&mut self, entity: Entity) -> u32 {
        let mut id = rand::random::<u32>();
        while self.entities.contains_key(&id) {
            id = rand::random::<u32>();
        }
        self.entities.insert(id, entity);
        id
    }

    pub fn get_entity(&self, id: u32) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, settings: &Settings){
        let loaders: Vec<_> = self.entities.iter().map(|(_id, e)| (e.x, e.y)).collect();

        for rb in &mut self.rigidbodies {
            let rb_w = rb.width;
            let rb_h = rb.height;

            if let Some(body) = &mut rb.body {
                let s = body.get_angle().sin();
                let c = body.get_angle().cos();
                let pos_x = body.get_position().x * LIQUIDFUN_SCALE;
                let pos_y = body.get_position().y * LIQUIDFUN_SCALE;

                for rb_y in 0..rb_w {
                    for rb_x in 0..rb_h {
                        let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                        let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                        let cur = rb.pixels[(rb_x + rb_y * rb_w) as usize];
                        if cur.material_id != AIR.id {
                            let world = self.chunk_handler.get(tx as i64, ty as i64);
                            if let Ok(mat) = world {
                                if mat.material_id == AIR.id {
                                    let _ignore = self.chunk_handler.set(tx as i64, ty as i64, MaterialInstance {
                                        physics: PhysicsType::Object,
                                        ..cur
                                    });
                                }else if mat.physics == PhysicsType::Sand {
                                    // let local_point = Vec2::new(f32::from(rb_x) / f32::from(rb_w), f32::from(rb_y) / f32::from(rb_h));
                                    let world_point = Vec2::new(tx / LIQUIDFUN_SCALE, ty / LIQUIDFUN_SCALE);

                                    let point_velocity: Vec2 = body.get_linear_velocity_from_world_point(&Vec2::new(tx / LIQUIDFUN_SCALE, ty / LIQUIDFUN_SCALE));
                                    // TODO: extract constant into material property (like weight or something)
                                    // TODO: consider making it so the body actually comes to a stop
                                    body.apply_force(&Vec2::new(-point_velocity.x * 0.1, -point_velocity.y * 0.1), &world_point, true);

                                    // let linear_velocity = body.get_linear_velocity();
                                    // body.set_linear_velocity(&Vec2::new(linear_velocity.x * 0.9999, linear_velocity.y * 0.9999));

                                    // let angular_velocity = body.get_angular_velocity();
                                    // body.set_angular_velocity(angular_velocity * 0.999);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.chunk_handler.tick(tick_time, &loaders, settings);

        for rb in &self.rigidbodies {
            let rb_w = rb.width;
            let rb_h = rb.height;
            let body_opt = rb.body.as_ref();

            if body_opt.is_some() {
                let s = body_opt.unwrap().get_angle().sin();
                let c = body_opt.unwrap().get_angle().cos();
                let pos_x = body_opt.unwrap().get_position().x * LIQUIDFUN_SCALE;
                let pos_y = body_opt.unwrap().get_position().y * LIQUIDFUN_SCALE;

                for rb_y in 0..rb_w {
                    for rb_x in 0..rb_h {
                        let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                        let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                        let world = self.chunk_handler.get(tx as i64, ty as i64);
                        if let Ok(mat) = world {
                            if mat.physics == PhysicsType::Object {
                                let _ignore = self.chunk_handler.set(tx as i64, ty as i64, MaterialInstance::air());
                            }
                        }
                    }
                }
            }
        }

        simulator::Simulator::simulate_rigidbodies(&mut self.chunk_handler, &mut self.rigidbodies, &mut self.lqf_world);
        
        for c in self.chunk_handler.loaded_chunks.borrow_mut().values_mut() {
            if c.get_b2_body().is_none() {
                // if let Some(tr) = c.get_tris() {
                //     let mut body_def = BodyDef::default();
                //     body_def.position.set((c.get_chunk_x() * CHUNK_SIZE as i32) as f32 / LIQUIDFUN_SCALE, (c.get_chunk_y() * CHUNK_SIZE as i32) as f32 / LIQUIDFUN_SCALE);
                //     let body = self.lqf_world.create_body(&body_def);
                        
                //     tr.iter().for_each(|tris| {
                //         tris.iter().for_each(|tri| {
                //             let mut poly = PolygonShape::new();

                //             let points = vec![
                //                 (tri.0.0 as f32 / LIQUIDFUN_SCALE, tri.0.1 as f32 / LIQUIDFUN_SCALE),
                //                 (tri.1.0 as f32 / LIQUIDFUN_SCALE, tri.1.1 as f32 / LIQUIDFUN_SCALE),
                //                 (tri.2.0 as f32 / LIQUIDFUN_SCALE, tri.2.1 as f32 / LIQUIDFUN_SCALE),
                //             ];

                //             poly.set(points);
                //             body.create_fixture_from_shape(&poly, 0.0);
                //         });
                //     });

                //     c.set_b2_body(Some(body));
                // }

                if let Some(loops) = c.get_mesh_loops() {
                    let mut body_def = BodyDef::default();
                    body_def.position.set((c.get_chunk_x() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE, (c.get_chunk_y() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE);
                    let mut body = self.lqf_world.create_body(&body_def);
                    body.set_active(false);

                    for a_loop in loops.iter() {
                        for pts in a_loop.iter() {
                            let mut verts: Vec<Vec2> = Vec::new();

                            for p in pts.iter() {
                                verts.push(Vec2::new(p[0] as f32 / LIQUIDFUN_SCALE, p[1] as f32 / LIQUIDFUN_SCALE));
                            }

                            let mut chain = ChainShape::new();
                            chain.create_chain(&verts, verts.len() as i32);
                            body.create_fixture_from_shape(&chain, 0.0);
                        }

                    }

                    c.set_b2_body(Some(body));
                }
            }else {
                // TODO: profile this and if it's too slow, could stagger it based on tick_time

                let chunk_center_x = c.get_chunk_x() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;
                let chunk_center_y = c.get_chunk_y() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;

                let dist_particle = f32::from(CHUNK_SIZE) * 0.6;
                let dist_body = f32::from(CHUNK_SIZE) * 1.0;

                let mut should_be_active = false;

                let mut psl = self.lqf_world.get_particle_system_list();
                while psl.is_some() && !should_be_active {
                    let system = psl.unwrap();
                    if system.get_position_buffer().iter().any(|pos| (pos.x * LIQUIDFUN_SCALE as f32 - chunk_center_x as f32).abs() < dist_particle && (pos.y * LIQUIDFUN_SCALE as f32 - chunk_center_y as f32).abs() < dist_particle) {
                        should_be_active = true;
                    }
                    psl = system.get_next();
                }

                // TODO: see if using box2d's query methods instead of direct iteration is faster
                let mut bl = self.lqf_world.get_body_list();
                while bl.is_some() && !should_be_active {
                    let body = bl.unwrap();

                    match body.get_type() {
                        BodyType::DynamicBody => {
                            // if body.is_awake() { // this just causes flickering
                            let pos = body.get_position();
                            let dist_x = (pos.x * LIQUIDFUN_SCALE as f32 - chunk_center_x as f32).abs();
                            let dist_y = (pos.y * LIQUIDFUN_SCALE as f32 - chunk_center_y as f32).abs();
                            if dist_x < dist_body && dist_y < dist_body {
                                should_be_active = true;
                            }
                            // }
                        },
                        BodyType::KinematicBody | BodyType::StaticBody => {},
                    }

                    bl = body.get_next();
                }

                if let Some(b) = c.get_b2_body_mut() {
                    b.set_active(should_be_active);
                }
            }
        }

        // match self.net_mode {
        //     WorldNetworkMode::Local => {
        //         self.chunk_handler.tick(tick_time, loaders, settings);
        //     },
        //     WorldNetworkMode::Remote => {},
        // }

        self.chunk_handler.update_chunk_graphics();
    }

    pub fn tick_lqf(&mut self, settings: &Settings) {

        // need to do this here since 'self' isn't mut in render
        if settings.lqf_dbg_draw {
            if let Some(cast) = self.lqf_world.get_debug_draw() {
                unsafe {
                    cast.SetFlags(0);
                    if settings.lqf_dbg_draw_shape {
                        cast.AppendFlags(b2draw::b2Draw_e_shapeBit as u32);
                    }
                    if settings.lqf_dbg_draw_joint {
                        cast.AppendFlags(b2draw::b2Draw_e_jointBit as u32);
                    }
                    if settings.lqf_dbg_draw_aabb {
                        cast.AppendFlags(b2draw::b2Draw_e_aabbBit as u32);
                    }
                    if settings.lqf_dbg_draw_pair {
                        cast.AppendFlags(b2draw::b2Draw_e_pairBit as u32);
                    }
                    if settings.lqf_dbg_draw_center_of_mass {
                        cast.AppendFlags(b2draw::b2Draw_e_centerOfMassBit as u32);
                    }
                    if settings.lqf_dbg_draw_particle {
                        cast.AppendFlags(b2draw::b2Draw_e_particleBit as u32);
                    }
                }
            }
        }


        let time_step = settings.tick_lqf_timestep;
        let velocity_iterations = 5;
        let position_iterations = 3;
        self.lqf_world.step(time_step, velocity_iterations, position_iterations);
        // match self.net_mode {
        //     WorldNetworkMode::Local => {
        //         let time_step = settings.tick_lqf_timestep;
        //         let velocity_iterations = 3;
        //         let position_iterations = 2;
        //         self.lqf_world.step(time_step, velocity_iterations, position_iterations);
        //     },
        //     WorldNetworkMode::Remote => {},
        // }
    }
}

