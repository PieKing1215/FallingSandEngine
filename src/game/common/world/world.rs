
use std::collections::HashMap;

use crate::game::common::Settings;
use liquidfun::box2d::{collision::shapes::polygon_shape::PolygonShape, common::{b2draw, math::Vec2}, dynamics::{body::{BodyDef, BodyType}, fixture::FixtureDef}, particle::{ParticleDef, TENSILE_PARTICLE, particle_system::ParticleSystemDef}};

use super::{ChunkHandler, entity::Entity, gen::{TEST_GENERATOR, TestGenerator}};

pub const LIQUIDFUN_SCALE: f32 = 10.0;

pub struct World {
    pub chunk_handler: ChunkHandler<TestGenerator>,
    pub lqf_world: liquidfun::box2d::dynamics::world::World,
    pub entities: HashMap<u32, Entity>,
}

impl<'w> World {
    #[profiling::function]
    pub fn create() -> Self {
        let gravity = liquidfun::box2d::common::math::Vec2::new(0.0, 3.0);
        let mut lqf_world = liquidfun::box2d::dynamics::world::World::new(&gravity);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(0.0, -26.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(46.0, 0.4);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(0.0, 0.4);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(12.0, 0.4);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(12.0, -6.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(0.4, 6.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(-12.0, -6.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(0.4, 6.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(7.0, -8.3);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(0.2, 8.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut body_def = BodyDef::default();
        body_def.body_type = BodyType::DynamicBody;
        body_def.position.set(0.0, -25.0);
        body_def.angular_velocity = 2.0;
        body_def.linear_velocity = Vec2::new(0.0, -4.0);
        let body = lqf_world.create_body(&body_def);
        let mut dynamic_box = PolygonShape::new();
        dynamic_box.set_as_box(1.0, 1.0);
        let mut fixture_def = FixtureDef::new(&dynamic_box);
        fixture_def.density = 1.5;
        fixture_def.friction = 0.3;
        body.create_fixture(&fixture_def);

        let mut body_def = BodyDef::default();
        body_def.body_type = BodyType::DynamicBody;
        body_def.position.set(-10.0, -25.0);
        body_def.angular_velocity = 2.0;
        body_def.linear_velocity = Vec2::new(0.0, -4.0);
        let body = lqf_world.create_body(&body_def);
        let mut dynamic_box = PolygonShape::new();
        dynamic_box.set_as_box(1.0, 1.0);
        let mut fixture_def = FixtureDef::new(&dynamic_box);
        fixture_def.density = 0.75;
        fixture_def.friction = 0.3;
        body.create_fixture(&fixture_def);

        // bottom section

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(0.0, 15.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(24.0, 0.4);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(35.0, -5.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box_oriented(0.4, 24.0, &Vec2{x: 0.0, y: 0.0}, 0.5);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(-35.0, -5.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box_oriented(0.4, 24.0, &Vec2{x: 0.0, y: 0.0}, -0.5);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);


        let mut particle_system_def = ParticleSystemDef::default();
        particle_system_def.radius = 0.19;
        particle_system_def.surface_tension_pressure_strength = 0.1;
        particle_system_def.surface_tension_normal_strength = 0.1;
        particle_system_def.damping_strength = 0.001;
	    let particle_system = lqf_world.create_particle_system(&particle_system_def);
        let mut pd = ParticleDef::default();
        pd.flags.insert(TENSILE_PARTICLE);
        pd.color.set(255, 90, 255, 255);

        for i in 0..2500 {
            if i < 12500 {
                pd.color.set(255, 200, 64, 191);
            }else {
                pd.color.set(64, 200, 255, 191);
            }
            pd.position.set(-25.0 + (i as f32 / 100.0) * 0.17, -6.0 - ((i % 100) as f32) * 0.17);
            particle_system.create_particle(&pd);
        }

        World {
            chunk_handler: ChunkHandler::new(TEST_GENERATOR),
            lqf_world,
            entities: HashMap::new(),
        }
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
        let loaders = self.entities.iter().map(|(id, e)| (e.x, e.y)).collect();
        self.chunk_handler.tick(tick_time, loaders, settings);
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
        let velocity_iterations = 3;
        let position_iterations = 2;
        self.lqf_world.step(time_step, velocity_iterations, position_iterations);
    }
}

