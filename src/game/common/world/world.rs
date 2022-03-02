use std::{borrow::BorrowMut, convert::Infallible, path::PathBuf, time::Duration};

use crate::game::common::{Settings, world::RigidBodyState};

use liquidfun::box2d::{
    collision::shapes::chain_shape::ChainShape,
    common::{b2draw, math::Vec2},
    dynamics::{
        body::{BodyDef, BodyType},
        fixture::FixtureDef,
    },
};
use rapier2d::{na::{Vector2, Point3, Point2, Isometry2, vector}, prelude::{RigidBodySet, ColliderSet, JointSet, RigidBodyBuilder, ColliderBuilder, SharedShape, IntegrationParameters, PhysicsPipeline, IslandManager, BroadPhase, NarrowPhase, CCDSolver, PhysicsHooks, EventHandler}};
use salva2d::{integrations::rapier::{FluidsPipeline, ColliderSampling}, solver::{Becker2009Elasticity, XSPHViscosity}, object::{Fluid, Boundary}};
use sdl2::pixels::Color;
use specs::{
    saveload::{MarkedBuilder, SimpleMarker, SimpleMarkerAllocator},
    Builder, Entities, Join, ReadStorage, RunNow, WorldExt, Write, WriteStorage, Read,
};

use super::{
    entity::{
        CollisionDetector, GameEntity, Hitbox, Persistent, PhysicsEntity, Player,
        UpdatePhysicsEntities,
    },
    gen::{TestGenerator, TEST_GENERATOR},
    material::{MaterialInstance, PhysicsType, AIR, TEST_MATERIAL},
    particle::{Particle, UpdateParticles, ParticleSystem},
    rigidbody::FSRigidBody,
    simulator, ApplyB2Bodies, AutoTarget, B2BodyComponent, Camera, Chunk, ChunkHandler,
    ChunkHandlerGeneric, CollisionFlags, DeltaTime, FilePersistent, Loader, Position, TickTime,
    UpdateAutoTargets, UpdateB2Bodies, Velocity, CHUNK_SIZE,
};

pub const LIQUIDFUN_SCALE: f32 = 10.0;

#[derive(Debug)]
pub enum WorldNetworkMode {
    Local,
    Remote,
}

pub struct World<C: Chunk> {
    pub ecs: specs::World,
    pub path: Option<PathBuf>,
    pub chunk_handler: ChunkHandler<TestGenerator, C>,
    pub lqf_world: liquidfun::box2d::dynamics::world::World,
    pub net_mode: WorldNetworkMode,
    pub rigidbodies: Vec<FSRigidBody>,
    pub physics: Physics,
}

pub struct Physics {
    pub fluid_pipeline: FluidsPipeline,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub gravity: Vector2<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub islands: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub joints: JointSet,
    pub hooks: Box<dyn PhysicsHooks<RigidBodySet, ColliderSet>>,
    pub event_handler: Box<dyn EventHandler>,
}

impl Physics {
    pub fn step(&mut self, time_step: f32) {
        self.fluid_pipeline.step(
            &self.gravity,
            time_step,
            &self.colliders,
            &mut self.bodies,
        );

        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.ccd_solver,
            &*self.hooks,
            &*self.event_handler,
        );

    }
}

const PARTICLE_RADIUS: f32 = 0.19;
const SMOOTHING_FACTOR: f32 = 2.0;

impl<'w, C: Chunk> World<C> {
    #[profiling::function]
    pub fn create(path: Option<PathBuf>) -> Self {
        let gravity = liquidfun::box2d::common::math::Vec2::new(0.0, 3.0);
        let lqf_world = liquidfun::box2d::dynamics::world::World::new(&gravity);

        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let joints = JointSet::new();
        let mut fluid_pipeline = FluidsPipeline::new(PARTICLE_RADIUS, SMOOTHING_FACTOR);

        // let mut points1: Vec<Point2<f32>> = Vec::new();
        // let mut points2 = Vec::new();
        // let ni = 25;
        // let nj = 15;
        // for i in 0..ni / 2 {
        //     for j in 0..nj {
        //         let x = (i as f32) * PARTICLE_RADIUS * 2.0 - ni as f32 * PARTICLE_RADIUS;
        //         let y = (j as f32 + 1.0) * PARTICLE_RADIUS * 2.0 - 10.0;
        //         points1.push(Point2::new(x, y));
        //         points2.push(Point2::new(x + ni as f32 * PARTICLE_RADIUS, y));
        //     }
        // }

        // for i in 0..100 {
        //     for j in -10..nj {
        //         let x = (i as f32) * PARTICLE_RADIUS * 4.0 - 25.0 - ni as f32 * PARTICLE_RADIUS;
        //         let y = (j as f32 + 1.0) * PARTICLE_RADIUS * 2.0 - 20.0;
        //         points2.push(Point2::new(x + ni as f32 * PARTICLE_RADIUS, y));
        //     }
        // }

        // let elasticity: Becker2009Elasticity = Becker2009Elasticity::new(1_000.0, 0.3, true);
        // let viscosity = XSPHViscosity::new(0.5, 1.0);
        // let mut fluid = Fluid::new(points1, PARTICLE_RADIUS, 1.0);
        // fluid.nonpressure_forces.push(Box::new(elasticity));
        // fluid.nonpressure_forces.push(Box::new(viscosity.clone()));
        // let fluid_handle = fluid_pipeline.liquid_world.add_fluid(fluid);

        // // let viscosity = XSPHViscosity::new(0.5, 1.0);
        // let mut fluid = Fluid::new(points2, PARTICLE_RADIUS, 1.0);
        // // fluid.nonpressure_forces.push(Box::new(viscosity.clone()));
        // let fluid_handle = fluid_pipeline.liquid_world.add_fluid(fluid);

        let rigid_body = RigidBodyBuilder::new_static().position(Isometry2::new(Vector2::new(0.0, 20.0), 0.0)).build();
        let handle = bodies.insert(rigid_body);
        let collider = ColliderBuilder::cuboid(10.0, 1.0).build();
        let co_handle = colliders.insert_with_parent(collider, handle, &mut bodies);
        let bo_handle = fluid_pipeline
            .liquid_world
            .add_boundary(Boundary::new(Vec::new()));
        fluid_pipeline.coupling.register_coupling(
            bo_handle,
            co_handle,
            ColliderSampling::DynamicContactSampling,
        );

        let integration_parameters = IntegrationParameters::default();
        let mut physics_pipeline = PhysicsPipeline::new();
        let mut islands = IslandManager::new();
        let mut broad_phase = BroadPhase::new();
        let mut narrow_phase = NarrowPhase::new();
        let mut ccd_solver = CCDSolver::new();
        let mut joints = JointSet::new();

        let phys = Physics {
            fluid_pipeline,
            bodies,
            colliders,
            gravity: Vector2::y() * 3.0,
            integration_parameters,
            physics_pipeline,
            islands,
            broad_phase,
            narrow_phase,
            ccd_solver,
            joints,
            hooks: Box::new(()),
            event_handler: Box::new(()),
        };

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

        // let particle_system_def = ParticleSystemDef {
        //     radius: 0.19,
        //     surface_tension_pressure_strength: 0.1,
        //     surface_tension_normal_strength: 0.1,
        //     damping_strength: 0.001,
        //     ..ParticleSystemDef::default()
        // };
        // let particle_system = lqf_world.create_particle_system(&particle_system_def);
        // let mut pd = ParticleDef::default();
        // pd.flags.insert(TENSILE_PARTICLE);
        // pd.color.set(255, 90, 255, 255);

        // for i in 0..15000 {
        //     if i < 15000/2 {
        //         pd.color.set(255, 200, 64, 191);
        //     }else {
        //         pd.color.set(64, 200, 255, 191);
        //     }
        //     pd.position.set(-7.0 + (i as f32 / 200.0) * 0.17, -6.0 - ((i % 200) as f32) * 0.17);
        //     particle_system.create_particle(&pd);
        // }

        let mut ecs = specs::World::new();
        ecs.register::<SimpleMarker<FilePersistent>>();
        ecs.insert(SimpleMarkerAllocator::<FilePersistent>::default());
        ecs.insert(DeltaTime(Duration::from_millis(1)));
        ecs.insert(TickTime(0));
        ecs.insert(ParticleSystem::default());
        ecs.register::<Position>();
        ecs.register::<Velocity>();
        ecs.register::<GameEntity>();
        ecs.register::<Loader>();
        ecs.register::<Player>();
        ecs.register::<PhysicsEntity>();
        ecs.register::<Hitbox>();
        ecs.register::<AutoTarget>();
        ecs.register::<Camera>();
        ecs.register::<Persistent>();
        ecs.register::<B2BodyComponent>();
        ecs.register::<CollisionDetector>();

        if let Some(path) = &path {
            let particles_path = path.join("particles.dat");
            if particles_path.exists() {
                match std::fs::File::open(particles_path.clone()) {
                    Ok(f) => {
                        match bincode::deserialize_from(f) {
                            Ok(ps) => {
                                let ps: ParticleSystem = ps;
                                *ecs.write_resource::<ParticleSystem>() = ps;
                            },
                            Err(e) => {
                                log::error!(
                                    "Failed to read particles from file @ {:?}: {:?}",
                                    particles_path,
                                    e
                                );
                            },
                        }

                        let (particle_system,) = ecs.system_data::<(Read<ParticleSystem>,)>();
                        log::debug!("Loaded {}/{} particles.", particle_system.active.len(), particle_system.sleeping.len());
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to open particles file for reading @ {:?}: {:?}",
                            particles_path,
                            e
                        );
                    }
                };

                ecs.maintain();
            } else {
                log::error!("Particles file missing @ {:?}", particles_path);
            }
        }

        let mut w = World {
            ecs,
            chunk_handler: ChunkHandler::new(TEST_GENERATOR, path.clone()),
            path,
            lqf_world,
            net_mode: WorldNetworkMode::Local,
            rigidbodies: Vec::new(),
            physics: phys,
        };

        // add a rigidbody

        let pixels: Vec<_> = (0..40 * 40)
            .map(|i| {
                let x: i32 = i % 40;
                let y: i32 = i / 40;
                if (x - 20).abs() < 5 || (y - 20).abs() < 5 {
                    MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Solid,
                        color: Color::RGB(
                            64,
                            if (x + y) % 4 >= 2 { 191 } else { 64 },
                            if (x + y) % 4 > 2 { 64 } else { 191 },
                        ),
                    }
                } else {
                    MaterialInstance::air()
                }
            })
            .collect();

        if let Ok(mut r) = FSRigidBody::make_bodies(&pixels, 40, 40, &mut w.physics, (-1.0, -7.0)) {
            w.rigidbodies.append(&mut r);
        }

        // add another rigidbody

        let pixels: Vec<_> = (0..40 * 40)
            .map(|i| {
                let x: i32 = i % 40;
                let y: i32 = i / 40;
                let dst = (x - 20) * (x - 20) + (y - 20) * (y - 20);
                if dst <= 10 * 10 {
                    MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Sand,
                        color: Color::RGB(255, 64, 255),
                    }
                } else if dst <= 20 * 20 && ((x - 20).abs() >= 5 || y > 20) {
                    MaterialInstance {
                        material_id: TEST_MATERIAL.id,
                        physics: PhysicsType::Solid,
                        color: Color::RGB(
                            if (x + y) % 4 >= 2 { 191 } else { 64 },
                            if (x + y) % 4 > 2 { 64 } else { 191 },
                            64,
                        ),
                    }
                } else {
                    MaterialInstance::air()
                }
            })
            .collect();

        if let Ok(mut r) = FSRigidBody::make_bodies(&pixels, 40, 40, &mut w.physics, (2.0, -6.5)) {
            w.rigidbodies.append(&mut r);
        }

        for n in 0..4 {
            // add more rigidbodies

            let pixels: Vec<_> = (0..30 * 30)
                .map(|i| {
                    let x: i32 = i % 30 + (((i + n * 22) as f32 / 60.0).sin() * 2.0) as i32;
                    let y: i32 = i / 30;
                    let dst = (x - 15) * (x - 15) + (y - 15) * (y - 15);
                    if dst > 5 * 5 && dst <= 10 * 10 {
                        MaterialInstance {
                            material_id: TEST_MATERIAL.id,
                            physics: PhysicsType::Solid,
                            color: Color::RGB(
                                if (x + y) % 4 >= 2 { 191 } else { 64 },
                                if (x + y) % 4 > 2 { 64 } else { 191 },
                                if (x + y) % 4 >= 2 { 191 } else { 64 },
                            ),
                        }
                    } else {
                        MaterialInstance::air()
                    }
                })
                .collect();

            if let Ok(mut r) = FSRigidBody::make_bodies(
                &pixels,
                30,
                30,
                &mut w.physics,
                (5.0 + n as f32 * 2.0, -7.0 + n as f32 * -0.75),
            ) {
                w.rigidbodies.append(&mut r);
            }
        }

        w
    }

    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.chunk_handler.unload_all_chunks()?;

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = &self.path {
            let particles_path = path.join("particles.dat");

            match std::fs::File::create(particles_path.clone()) {
                Ok(f) => {
                    if let Err(e) = bincode::serialize_into(f, &*self.ecs.read_resource::<ParticleSystem>()) {
                        log::error!(
                            "Failed to write particles to file @ {:?}: {:?}",
                            particles_path,
                            e
                        );
                    }
                }
                Err(e) => {
                    log::error!(
                        "Failed to open particles file for writing @ {:?}: {:?}",
                        particles_path,
                        e
                    );
                }
            };
        }

        self.chunk_handler.save_all_chunks()?;

        Ok(())
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, settings: &Settings) {
        *self.ecs.write_resource::<TickTime>() = TickTime(tick_time);

        {
            profiling::scope!("fill rigidbodies");
            for rb_i in 0..self.rigidbodies.len() {
                let rb = &mut self.rigidbodies[rb_i];
                let rb_w = rb.width;
                let rb_h = rb.height;

                if let Some(body) = rb.get_body_mut(&mut self.physics) {
                    let s = body.rotation().angle().sin();
                    let c = body.rotation().angle().cos();
                    let pos_x = body.translation().x * LIQUIDFUN_SCALE;
                    let pos_y = body.translation().y * LIQUIDFUN_SCALE;

                    for rb_y in 0..rb_w {
                        for rb_x in 0..rb_h {
                            let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                            let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                            let cur = rb.pixels[(rb_x + rb_y * rb_w) as usize];
                            if cur.material_id != AIR.id {
                                let world = self.chunk_handler.get(tx as i64, ty as i64);
                                if let Ok(mat) = world {
                                    if mat.material_id == AIR.id {
                                        let _ignore = self.chunk_handler.set(
                                            tx as i64,
                                            ty as i64,
                                            MaterialInstance { physics: PhysicsType::Object, ..cur },
                                        );
                                    } else if mat.physics == PhysicsType::Sand {
                                        // let local_point = Vec2::new(f32::from(rb_x) / f32::from(rb_w), f32::from(rb_y) / f32::from(rb_h));
                                        let world_point =
                                            Point2::new(tx / LIQUIDFUN_SCALE, ty / LIQUIDFUN_SCALE);

                                        let point_velocity = body
                                            .velocity_at_point(&Point2::new(
                                                tx / LIQUIDFUN_SCALE,
                                                ty / LIQUIDFUN_SCALE,
                                            ));
                                        // TODO: extract constant into material property (like weight or something)
                                        // TODO: consider making it so the body actually comes to a stop
                                        body.apply_force_at_point(
                                            Vector2::new(
                                                -point_velocity.x * 0.1,
                                                -point_velocity.y * 0.1,
                                            ),
                                            world_point,
                                            true,
                                        );

                                        if point_velocity.x.abs() > 1.0 || point_velocity.y.abs() > 1.0
                                        {
                                            let m = *mat;
                                            let part_pos =
                                                Position { x: f64::from(tx), y: f64::from(ty) };
                                            let mut part_vel = Velocity {
                                                x: f64::from(point_velocity.x * 0.1),
                                                y: f64::from(point_velocity.y * 0.1 - 0.5),
                                            };

                                            let res = self.chunk_handler.set(
                                                tx as i64,
                                                ty as i64,
                                                MaterialInstance {
                                                    physics: PhysicsType::Object,
                                                    ..cur
                                                },
                                            );

                                            if res.is_ok() {
                                                match self.chunk_handler.get(
                                                    (part_pos.x + part_vel.x) as i64,
                                                    (part_pos.y + part_vel.y) as i64,
                                                ) {
                                                    Ok(m_test)
                                                        if m_test.physics != PhysicsType::Air =>
                                                    {
                                                        part_vel.x *= -1.0;
                                                        part_vel.y *= -1.0;

                                                        let part = Particle::new(
                                                            m,
                                                            part_pos,
                                                            part_vel,
                                                        );
                                                        self.ecs.write_resource::<ParticleSystem>().active.push(part);

                                                        body.apply_force_at_point(
                                                            Vector2::new(
                                                                -point_velocity.x * 0.5,
                                                                -point_velocity.y * 0.5,
                                                            ),
                                                            world_point,
                                                            true,
                                                        );

                                                        let linear_velocity =
                                                            *body.linvel();
                                                        body.set_linvel(Vector2::new(
                                                            linear_velocity.x * 0.999,
                                                            linear_velocity.y * 0.999,
                                                        ), true);

                                                        let angular_velocity =
                                                            body.angvel();
                                                        body.set_angvel(
                                                            angular_velocity * 0.999, true
                                                        );
                                                    }
                                                    _ => {
                                                        if !self
                                                            .chunk_handler
                                                            .displace(tx as i64, ty as i64, m)
                                                        {
                                                            let part = Particle::new(
                                                                m,
                                                                part_pos,
                                                                part_vel,
                                                            );
                                                            self.ecs.write_resource::<ParticleSystem>().active.push(part);

                                                            body.apply_force_at_point(
                                                                Vector2::new(
                                                                    -point_velocity.x * 0.75,
                                                                    -point_velocity.y * 0.75,
                                                                ),
                                                                world_point,
                                                                true,
                                                            );

                                                            let linear_velocity =
                                                                *body.linvel();
                                                            body.set_linvel(Vector2::new(
                                                                linear_velocity.x * 0.9,
                                                                linear_velocity.y * 0.9,
                                                            ), true);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            body.apply_force_at_point(
                                                Vector2::new(
                                                    -point_velocity.x * 0.1,
                                                    -point_velocity.y * 0.1,
                                                ),
                                                world_point,
                                                true,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        {
            profiling::scope!("fill Objects");

            let (position_storage, velocity_storage, phys_ent_storage, hitbox_storage) =
                self.ecs.system_data::<(
                    ReadStorage<Position>,
                    ReadStorage<Velocity>,
                    ReadStorage<PhysicsEntity>,
                    ReadStorage<Hitbox>,
                )>();

            // need this since using self.chunk_handler inside the closure doesn't work
            let ch = &mut self.chunk_handler;

            // let mut create_particles: Vec<(Particle, Position, Velocity)> = vec![];

            (
                &position_storage,
                &velocity_storage,
                &phys_ent_storage,
                &hitbox_storage,
            )
                .join()
                .for_each(|(pos, _vel, _phys_ent, hitbox)| {
                    let steps_x = ((hitbox.x2 - hitbox.x1).signum()
                        * (hitbox.x2 - hitbox.x1).abs().ceil())
                        as u16;
                    let steps_y = ((hitbox.y2 - hitbox.y1).signum()
                        * (hitbox.y2 - hitbox.y1).abs().ceil())
                        as u16;

                    let r: Vec<(f32, f32)> = (0..=steps_x)
                        .flat_map(move |a| (0..=steps_y).map(move |b| (a, b)))
                        .map(|(xs, ys)| {
                            (
                                (f32::from(xs) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1)
                                    + hitbox.x1,
                                (f32::from(ys) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1)
                                    + hitbox.y1,
                            )
                        })
                        .collect();

                    for (dx, dy) in r {
                        let pos_x = pos.x + f64::from(dx);
                        let pos_y = pos.y + f64::from(dy);

                        let world = ch.get(pos_x.floor() as i64, pos_y.floor() as i64);
                        if let Ok(mat) = world.map(|m| *m) {
                            if mat.material_id == AIR.id {
                                let _ignore = ch.set(
                                    pos_x.floor() as i64,
                                    pos_y.floor() as i64,
                                    MaterialInstance {
                                        physics: PhysicsType::Object,
                                        color: Color::RGB(0, 255, 0),
                                        ..mat
                                    },
                                );
                            }
                            // else if mat.physics == PhysicsType::Sand && ch.set(pos_x as i64, pos_y as i64, MaterialInstance::air()).is_ok() {
                            //     create_particles.push((
                            //         Particle::of(mat),
                            //         Position { x: pos_x, y: pos_y },
                            //         Velocity { x: 0.0, y: 0.0 },
                            //     ));
                            // }
                        }
                    }
                });

            // drop(position_storage);
            // drop(velocity_storage);
            // drop(phys_ent_storage);
            // drop(hitbox_storage);

            // for (part, pos, vel) in create_particles {
            //     self.ecs.create_entity().with(part).with(pos).with(vel)
            //         .marked::<SimpleMarker<FilePersistent>>().build();
            // }
        }

        self.chunk_handler.tick(tick_time, settings, &mut self.ecs);

        if settings.simulate_particles {
            let mut update_particles = UpdateParticles { chunk_handler: &mut self.chunk_handler };
            update_particles.run_now(&self.ecs);
            self.ecs.maintain();
        }

        {
            profiling::scope!("unfill Objects");
            let (position_storage, velocity_storage, phys_ent_storage, hitbox_storage) =
                self.ecs.system_data::<(
                    ReadStorage<Position>,
                    ReadStorage<Velocity>,
                    ReadStorage<PhysicsEntity>,
                    ReadStorage<Hitbox>,
                )>();

            // need this since using self.chunk_handler inside the closure doesn't work
            let ch = &mut self.chunk_handler;

            (
                &position_storage,
                &velocity_storage,
                &phys_ent_storage,
                &hitbox_storage,
            )
                .join()
                .for_each(|(pos, _vel, _phys_ent, hitbox)| {
                    let steps_x = ((hitbox.x2 - hitbox.x1).signum()
                        * (hitbox.x2 - hitbox.x1).abs().ceil())
                        as u16;
                    let steps_y = ((hitbox.y2 - hitbox.y1).signum()
                        * (hitbox.y2 - hitbox.y1).abs().ceil())
                        as u16;

                    let r: Vec<(f32, f32)> = (0..=steps_x)
                        .flat_map(move |a| (0..=steps_y).map(move |b| (a, b)))
                        .map(|(xs, ys)| {
                            (
                                (f32::from(xs) / f32::from(steps_x)) * (hitbox.x2 - hitbox.x1)
                                    + hitbox.x1,
                                (f32::from(ys) / f32::from(steps_y)) * (hitbox.y2 - hitbox.y1)
                                    + hitbox.y1,
                            )
                        })
                        .collect();

                    for (dx, dy) in r {
                        let pos_x = pos.x + f64::from(dx);
                        let pos_y = pos.y + f64::from(dy);

                        let world = ch.get(pos_x.floor() as i64, pos_y.floor() as i64);
                        if let Ok(mat) = world {
                            if mat.physics == PhysicsType::Object {
                                let _ignore = ch.set(
                                    pos_x.floor() as i64,
                                    pos_y.floor() as i64,
                                    MaterialInstance::air(),
                                );
                            }
                        }
                    }
                });
        }

        let mut update_physics_entities =
            UpdatePhysicsEntities { chunk_handler: &mut self.chunk_handler };
        update_physics_entities.run_now(&self.ecs);
        self.ecs.maintain();

        {
            profiling::scope!("unfill rigidbodies");
            for rb in &self.rigidbodies {
                let rb_w = rb.width;
                let rb_h = rb.height;
                let body_opt = rb.get_body(&self.physics);

                if body_opt.is_some() {
                    let s = body_opt.unwrap().rotation().angle().sin();
                    let c = body_opt.unwrap().rotation().angle().cos();
                    let pos_x = body_opt.unwrap().translation().x * LIQUIDFUN_SCALE;
                    let pos_y = body_opt.unwrap().translation().y * LIQUIDFUN_SCALE;

                    for rb_y in 0..rb_w {
                        for rb_x in 0..rb_h {
                            let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                            let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                            let world = self.chunk_handler.get(tx as i64, ty as i64);
                            if let Ok(mat) = world {
                                if mat.physics == PhysicsType::Object {
                                    let _ignore = self.chunk_handler.set(
                                        tx as i64,
                                        ty as i64,
                                        MaterialInstance::air(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        {
            profiling::scope!("sim rigidbodies");
            let mut new_parts = Vec::new();
            simulator::Simulator::simulate_rigidbodies(
                &mut self.chunk_handler,
                &mut self.rigidbodies,
                &mut self.physics,
                &mut new_parts,
            );
            self.ecs.write_resource::<ParticleSystem>().active.append(&mut new_parts);
        }

        {
            profiling::scope!("update chunk collision");
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
                        body_def.position.set(
                            (c.get_chunk_x() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE,
                            (c.get_chunk_y() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE,
                        );
                        let mut body = self.lqf_world.create_body(&body_def);
                        body.set_active(false);

                        let mut rigid_body = RigidBodyBuilder::new_static().translation(Vector2::new(
                            (c.get_chunk_x() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE,
                            (c.get_chunk_y() * i32::from(CHUNK_SIZE)) as f32 / LIQUIDFUN_SCALE
                        )).build();
                        let mut colliders = Vec::new();

                        for a_loop in loops.iter() {
                            for pts in a_loop.iter() {
                                let mut verts: Vec<Vec2> = Vec::new();

                                for p in pts.iter() {
                                    verts.push(Vec2::new(
                                        p[0] as f32 / LIQUIDFUN_SCALE,
                                        p[1] as f32 / LIQUIDFUN_SCALE,
                                    ));
                                }

                                let mut chain = ChainShape::new();
                                #[allow(clippy::cast_possible_wrap)]
                                chain.create_chain(&verts, verts.len() as i32);

                                let mut fixture_def = FixtureDef::new(&chain);
                                fixture_def.density = 0.0;
                                fixture_def.filter.category_bits = CollisionFlags::WORLD.bits();
                                fixture_def.filter.mask_bits = CollisionFlags::RIGIDBODY.bits();
                                body.create_fixture(&fixture_def);

                                let collider = ColliderBuilder::polyline(verts.iter().map(|v| Point2::new(v.x, v.y)).collect(), None).build();
                                colliders.push(collider);
                            }
                        }

                        c.set_b2_body(Some(body));
                        c.set_rigidbody(Some(RigidBodyState::Inactive(rigid_body, colliders)));
                    }
                } else {
                    // TODO: profile this and if it's too slow, could stagger it based on tick_time

                    let chunk_center_x =
                        c.get_chunk_x() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;
                    let chunk_center_y =
                        c.get_chunk_y() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;

                    let dist_particle = f32::from(CHUNK_SIZE) * 0.6;
                    let dist_body = f32::from(CHUNK_SIZE) * 1.0;

                    let mut should_be_active = false;

                    let mut psl = self.lqf_world.get_particle_system_list();
                    while psl.is_some() && !should_be_active {
                        let system = psl.unwrap();
                        if system.get_position_buffer().iter().any(|pos| {
                            (pos.x * LIQUIDFUN_SCALE as f32 - chunk_center_x as f32).abs()
                                < dist_particle
                                && (pos.y * LIQUIDFUN_SCALE as f32 - chunk_center_y as f32).abs()
                                    < dist_particle
                        }) {
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
                                let dist_x =
                                    (pos.x * LIQUIDFUN_SCALE as f32 - chunk_center_x as f32).abs();
                                let dist_y =
                                    (pos.y * LIQUIDFUN_SCALE as f32 - chunk_center_y as f32).abs();
                                if dist_x < dist_body && dist_y < dist_body {
                                    should_be_active = true;
                                }
                                // }
                            }
                            BodyType::KinematicBody | BodyType::StaticBody => {}
                        }

                        bl = body.get_next();
                    }

                    if let Some(b) = c.get_b2_body_mut() {
                        b.set_active(should_be_active);
                    }

                    if let Some(state) = c.get_rigidbody_mut() {
                        match state {
                            RigidBodyState::Active(h) if !should_be_active => {
                                let cls = self.physics.bodies.get(*h).unwrap().colliders().iter().map(|h| *h).collect::<Vec<_>>();
                                let colls = cls.iter().map(|ch| self.physics.colliders.remove(*ch, &mut self.physics.islands, &mut self.physics.bodies, false).unwrap()).collect::<Vec<_>>();
                                let rb = self.physics.bodies.remove(*h, &mut self.physics.islands, &mut self.physics.colliders, &mut self.physics.joints).unwrap();
                                *state = RigidBodyState::Inactive(rb, colls);
                            },
                            _ => {},
                        }

                        if should_be_active && matches!(state, RigidBodyState::Inactive(_, _)) {
                            match c.get_rigidbody_mut().take().unwrap() {
                                RigidBodyState::Inactive(rb, colls) if should_be_active => {
                                    let rb_handle = self.physics.bodies.insert(rb);
                                    for collider in colls {
                                        let bo_handle = self.physics.fluid_pipeline
                                            .liquid_world
                                            .add_boundary(Boundary::new(Vec::new()));
                                        let co_handle = self.physics.colliders.insert_with_parent(collider, rb_handle, &mut self.physics.bodies);
                                        self.physics.fluid_pipeline.coupling.register_coupling(
                                            bo_handle,
                                            co_handle,
                                            ColliderSampling::DynamicContactSampling,
                                        );
                                    }
                                    c.set_rigidbody(Some(RigidBodyState::Active(rb_handle)));
                                },
                                _ => {},
                            }
                        }
                    }
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

        let mut update_bodies = UpdateB2Bodies;
        update_bodies.run_now(&self.ecs);

        let time_step = settings.tick_lqf_timestep;
        let velocity_iterations = 8;
        let position_iterations = 4;
        self.lqf_world
            .step(time_step, velocity_iterations, position_iterations);
        // match self.net_mode {
        //     WorldNetworkMode::Local => {
        //         let time_step = settings.tick_lqf_timestep;
        //         let velocity_iterations = 3;
        //         let position_iterations = 2;
        //         self.lqf_world.step(time_step, velocity_iterations, position_iterations);
        //     },
        //     WorldNetworkMode::Remote => {},
        // }

        self.physics.step(time_step / 3.0);
        self.physics.step(time_step / 3.0);
        self.physics.step(time_step / 3.0);

        let mut apply_bodies = ApplyB2Bodies;
        apply_bodies.run_now(&self.ecs);
    }

    pub fn frame(&mut self, delta_time: Duration) {
        *self.ecs.write_resource::<DeltaTime>() = DeltaTime(delta_time);

        let mut update_auto_targets = UpdateAutoTargets;
        update_auto_targets.run_now(&self.ecs);
    }

    pub fn raycast(
        &self,
        mut x1: i64,
        mut y1: i64,
        x2: i64,
        y2: i64,
        collide_filder: fn((i64, i64), &MaterialInstance) -> bool,
    ) -> Option<((i64, i64), &MaterialInstance)> {
        let check_pixel = |x: i64, y: i64| {
            let r = self.chunk_handler.get(x, y);
            if let Ok(m) = r {
                if m.physics != PhysicsType::Air {
                    return Some(((x, y), m));
                }
            }
            None
        };

        let x_dist = (x2 - x1).abs();
        let y_dist = -(y2 - y1).abs();
        let x_step = if x1 < x2 { 1 } else { -1 };
        let y_step = if y1 < y2 { 1 } else { -1 };
        let mut error = x_dist + y_dist;

        if let Some(r) = check_pixel(x1, y1) {
            if collide_filder(r.0, r.1) {
                return Some(r);
            }
        }

        while x1 != x2 || y1 != y2 {
            let tmp = 2 * error;

            if tmp > y_dist {
                error += y_dist;
                x1 += x_step;
            }

            if tmp < x_dist {
                error += x_dist;
                y1 += y_step;
            }

            if let Some(r) = check_pixel(x1, y1) {
                if collide_filder(r.0, r.1) {
                    return Some(r);
                }
            }
        }

        None
    }
}
