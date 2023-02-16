use std::{
    borrow::BorrowMut,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use crate::game::{
    common::{
        world::{physics::PHYSICS_SCALE, RigidBodyState},
        Settings,
    },
    Registries,
};

use rapier2d::{
    na::{Point2, Vector2},
    prelude::{ColliderBuilder, InteractionGroups, RigidBodyBuilder, RigidBodyType},
};
// use salva2d::{integrations::rapier::ColliderSampling, object::Boundary};
use specs::{
    saveload::{SimpleMarker, SimpleMarkerAllocator},
    Join, Read, ReadStorage, RunNow, WorldExt,
};

use super::{
    entity::{
        CollisionDetector, GameEntity, Hitbox, Persistent, PhysicsEntity, Player,
        UpdatePhysicsEntities,
    },
    gen::{biome_test::BiomeTestGenerator, structure::StructureNode},
    material::{self, color::Color, MaterialInstance, PhysicsType},
    particle::{Particle, ParticleSystem, UpdateParticles},
    physics::Physics,
    rigidbody::FSRigidBody,
    simulator, ApplyRigidBodies, AutoTarget, Camera, Chunk, ChunkHandler, ChunkHandlerGeneric,
    CollisionFlags, DeltaTime, FilePersistent, Loader, Position, RigidBodyComponent, TickTime,
    UpdateAutoTargets, UpdateRigidBodies, Velocity, CHUNK_SIZE,
};

#[derive(Debug)]
pub enum WorldNetworkMode {
    Local,
    Remote,
}

pub struct World<C: Chunk> {
    pub ecs: specs::World,
    pub path: Option<PathBuf>,
    pub chunk_handler: ChunkHandler<C>,
    pub net_mode: WorldNetworkMode,
    pub rigidbodies: Vec<FSRigidBody>,
    pub physics: Physics,
    pub seed: i32,
}

impl<C: Chunk + Send> World<C> {
    #[profiling::function]
    pub fn create(path: Option<PathBuf>, seed: Option<i32>) -> Self {
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
        ecs.register::<RigidBodyComponent>();
        ecs.register::<CollisionDetector>();
        ecs.register::<StructureNode>();

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
                        log::debug!(
                            "Loaded {}/{} particles.",
                            particle_system.active.len(),
                            particle_system.sleeping.len()
                        );
                    },
                    Err(e) => {
                        log::error!(
                            "Failed to open particles file for reading @ {:?}: {:?}",
                            particles_path,
                            e
                        );
                    },
                };

                ecs.maintain();
            } else {
                log::error!("Particles file missing @ {:?}", particles_path);
            }
        }

        let mut w = World {
            ecs,
            chunk_handler: ChunkHandler::new(BiomeTestGenerator::new(), path.clone()),
            path,
            net_mode: WorldNetworkMode::Local,
            rigidbodies: Vec::new(),
            physics: Physics::new(),
            seed: seed.unwrap_or_else(|| {
                let mut h = DefaultHasher::new();
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i32)
                    .hash(&mut h);
                h.finish() as i32
            }),
        };

        // add a rigidbody

        let pixels: Vec<_> = (0..40 * 40)
            .map(|i| {
                let x: i32 = i % 40;
                let y: i32 = i / 40;
                if (x - 20).abs() < 5 || (y - 20).abs() < 5 {
                    MaterialInstance {
                        material_id: material::TEST,
                        physics: PhysicsType::Solid,
                        color: Color::rgb(
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

        // asymmetric
        let pixels: Vec<_> = (0..40 * 40)
            .map(|i| {
                let x: i32 = i % 40;
                let y: i32 = i / 40;
                if (y <= 5) || ((x - y).abs() <= 5) {
                    MaterialInstance {
                        material_id: material::TEST,
                        physics: PhysicsType::Solid,
                        color: Color::rgb(
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

        if let Ok(mut r) = FSRigidBody::make_bodies(&pixels, 40, 40, &mut w.physics, (-0.0, -10.0))
        {
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
                        material_id: material::TEST,
                        physics: PhysicsType::Sand,
                        color: Color::rgb(255, 64, 255),
                    }
                } else if dst <= 20 * 20 && ((x - 20).abs() >= 5 || y > 20) {
                    MaterialInstance {
                        material_id: material::TEST,
                        physics: PhysicsType::Solid,
                        color: Color::rgb(
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
                            material_id: material::TEST,
                            physics: PhysicsType::Solid,
                            color: Color::rgb(
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
        self.chunk_handler.unload_all_chunks(&mut self.physics)?;

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = &self.path {
            let particles_path = path.join("particles.dat");

            match std::fs::File::create(particles_path.clone()) {
                Ok(f) => {
                    if let Err(e) =
                        bincode::serialize_into(f, &*self.ecs.read_resource::<ParticleSystem>())
                    {
                        log::error!(
                            "Failed to write particles to file @ {:?}: {:?}",
                            particles_path,
                            e
                        );
                    }
                },
                Err(e) => {
                    log::error!(
                        "Failed to open particles file for writing @ {:?}: {:?}",
                        particles_path,
                        e
                    );
                },
            };
        }

        self.chunk_handler.save_all_chunks()?;

        Ok(())
    }

    #[profiling::function]
    pub fn tick_physics(&mut self, settings: &Settings) {
        // need to do this here since 'self' isn't mut in render

        let mut update_bodies = UpdateRigidBodies { physics: &mut self.physics };
        update_bodies.run_now(&self.ecs);

        let time_step = settings.tick_physics_timestep;
        // match self.net_mode {
        //     WorldNetworkMode::Local => {
        //         let time_step = settings.tick_physics_timestep;
        //         let velocity_iterations = 3;
        //         let position_iterations = 2;
        //         self.lqf_world.step(time_step, velocity_iterations, position_iterations);
        //     },
        //     WorldNetworkMode::Remote => {},
        // }

        self.physics.step(time_step / 3.0);
        self.physics.step(time_step / 3.0);
        self.physics.step(time_step / 3.0);

        let mut apply_bodies = ApplyRigidBodies { physics: &mut self.physics };
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

impl<C: Chunk + Send + Sync> World<C> {
    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, settings: &Settings, registries: Arc<Registries>) {
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
                    let pos_x = body.translation().x * PHYSICS_SCALE;
                    let pos_y = body.translation().y * PHYSICS_SCALE;

                    let mut impediment = 0.0_f32;

                    for rb_y in 0..rb_w {
                        for rb_x in 0..rb_h {
                            let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                            let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                            let cur = rb.pixels[(rb_x + rb_y * rb_w) as usize];
                            if cur.material_id != material::AIR {
                                let world = self.chunk_handler.get(tx as i64, ty as i64);
                                if let Ok(mat) = world {
                                    if mat.material_id == material::AIR {
                                        let _ignore = self.chunk_handler.set(
                                            tx as i64,
                                            ty as i64,
                                            MaterialInstance {
                                                physics: PhysicsType::Object,
                                                ..cur
                                            },
                                        );
                                    } else if mat.physics == PhysicsType::Sand {
                                        // let local_point = Vec2::new(f32::from(rb_x) / f32::from(rb_w), f32::from(rb_y) / f32::from(rb_h));
                                        let world_point =
                                            Point2::new(tx / PHYSICS_SCALE, ty / PHYSICS_SCALE);

                                        let point_velocity = body.velocity_at_point(&Point2::new(
                                            tx / PHYSICS_SCALE,
                                            ty / PHYSICS_SCALE,
                                        ));
                                        // TODO: extract constant into material property (like weight or something)
                                        body.apply_impulse_at_point(
                                            Vector2::new(
                                                -point_velocity.x * 0.1 / body.mass(),
                                                -point_velocity.y * 0.1 / body.mass(),
                                            ),
                                            world_point,
                                            true,
                                        );

                                        if point_velocity.x.abs() > 1.0
                                            || point_velocity.y.abs() > 1.0
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

                                                        let part =
                                                            Particle::new(m, part_pos, part_vel);
                                                        self.ecs
                                                            .write_resource::<ParticleSystem>()
                                                            .active
                                                            .push(part);

                                                        body.apply_impulse_at_point(
                                                            Vector2::new(
                                                                -point_velocity.x * 0.25
                                                                    / body.mass(),
                                                                -point_velocity.y * 0.25
                                                                    / body.mass(),
                                                            ),
                                                            world_point,
                                                            true,
                                                        );

                                                        let linear_velocity = *body.linvel();
                                                        body.set_linvel(
                                                            Vector2::new(
                                                                linear_velocity.x * 0.999,
                                                                linear_velocity.y * 0.999,
                                                            ),
                                                            true,
                                                        );

                                                        let angular_velocity = body.angvel();
                                                        body.set_angvel(
                                                            angular_velocity * 0.999,
                                                            true,
                                                        );
                                                    },
                                                    _ => {
                                                        if !self
                                                            .chunk_handler
                                                            .displace(tx as i64, ty as i64, m)
                                                        {
                                                            let part = Particle::new(
                                                                m, part_pos, part_vel,
                                                            );
                                                            self.ecs
                                                                .write_resource::<ParticleSystem>()
                                                                .active
                                                                .push(part);

                                                            body.apply_impulse_at_point(
                                                                Vector2::new(
                                                                    -point_velocity.x * 0.75
                                                                        / body.mass(),
                                                                    -point_velocity.y * 0.75
                                                                        / body.mass(),
                                                                ),
                                                                world_point,
                                                                true,
                                                            );

                                                            let linear_velocity = *body.linvel();
                                                            body.set_linvel(
                                                                Vector2::new(
                                                                    linear_velocity.x * 0.1,
                                                                    linear_velocity.y * 0.1,
                                                                ),
                                                                true,
                                                            );
                                                        }
                                                    },
                                                }
                                            }
                                        } else {
                                            body.apply_impulse_at_point(
                                                Vector2::new(
                                                    -point_velocity.x * 1.0 / body.mass(),
                                                    -point_velocity.y * 1.0 / body.mass(),
                                                ),
                                                world_point,
                                                true,
                                            );
                                            impediment += 1.0 / 20.0; // TODO: this could be a material property
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // this gravity manipulation makes it so the body can come to a full stop in sand
                    // the if is to help with making sure the body is woken up by changes in impedement

                    let prev_gravity = body.gravity_scale();
                    let new_gravity = 1.0 - (impediment / body.mass()).clamp(0.0, 1.0);

                    // only wake and update if new gravity is different enough
                    // extra checks for 0.0 and 1.0 to make sure it doesn't get stuck at 0.01 and prevent sleeping
                    if (prev_gravity - new_gravity).abs() > 0.01
                        || (new_gravity == 0.0 && prev_gravity != 0.0)
                        || ((new_gravity - 1.0).abs() < 0.001 && (prev_gravity - 1.0).abs() > 0.001)
                    {
                        body.set_gravity_scale(new_gravity, true);
                        body.wake_up(true);
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
                            if mat.material_id == material::AIR {
                                let _ignore = ch.set(
                                    pos_x.floor() as i64,
                                    pos_y.floor() as i64,
                                    MaterialInstance {
                                        physics: PhysicsType::Object,
                                        color: Color::rgb(0, 255, 0),
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

        self.chunk_handler.tick(
            tick_time,
            settings,
            &mut self.ecs,
            &mut self.physics,
            registries,
            self.seed,
        );

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

                if let Some(body) = body_opt {
                    let (s, c) = body.rotation().angle().sin_cos();
                    let pos_x = body.translation().x * PHYSICS_SCALE;
                    let pos_y = body.translation().y * PHYSICS_SCALE;

                    for rb_y in 0..rb_w {
                        for rb_x in 0..rb_h {
                            let tx = f32::from(rb_x) * c - f32::from(rb_y) * s + pos_x;
                            let ty = f32::from(rb_x) * s + f32::from(rb_y) * c + pos_y;

                            // ok to fail since the chunk might just not be ready
                            let _ignore = self.chunk_handler.replace(tx as i64, ty as i64, |mat| {
                                (mat.physics == PhysicsType::Object).then(MaterialInstance::air)
                            });
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
            self.ecs
                .write_resource::<ParticleSystem>()
                .active
                .append(&mut new_parts);
        }

        {
            profiling::scope!("update chunk collision");
            for c in self.chunk_handler.loaded_chunks.borrow_mut().values_mut() {
                if c.get_rigidbody().is_none() {
                    // if let Some(tr) = c.get_tris() {
                    //     let mut body_def = BodyDef::default();
                    //     body_def.position.set((c.get_chunk_x() * CHUNK_SIZE as i32) as f32 / PHYSICS_SCALE, (c.get_chunk_y() * CHUNK_SIZE as i32) as f32 / PHYSICS_SCALE);
                    //     let body = self.lqf_world.create_body(&body_def);

                    //     tr.iter().for_each(|tris| {
                    //         tris.iter().for_each(|tri| {
                    //             let mut poly = PolygonShape::new();

                    //             let points = vec![
                    //                 (tri.0.0 as f32 / PHYSICS_SCALE, tri.0.1 as f32 / PHYSICS_SCALE),
                    //                 (tri.1.0 as f32 / PHYSICS_SCALE, tri.1.1 as f32 / PHYSICS_SCALE),
                    //                 (tri.2.0 as f32 / PHYSICS_SCALE, tri.2.1 as f32 / PHYSICS_SCALE),
                    //             ];

                    //             poly.set(points);
                    //             body.create_fixture_from_shape(&poly, 0.0);
                    //         });
                    //     });

                    //     c.set_b2_body(Some(body));
                    // }

                    if let Some(loops) = c.get_mesh_loops() {
                        let rigid_body = RigidBodyBuilder::fixed()
                            .translation(Vector2::new(
                                (c.get_chunk_x() * i32::from(CHUNK_SIZE)) as f32 / PHYSICS_SCALE,
                                (c.get_chunk_y() * i32::from(CHUNK_SIZE)) as f32 / PHYSICS_SCALE,
                            ))
                            .build();
                        let mut colliders = Vec::new();

                        for a_loop in loops.iter() {
                            for pts in a_loop.iter() {
                                let mut verts: Vec<Point2<f32>> = Vec::new();

                                for p in pts.iter() {
                                    verts.push(Point2::new(
                                        p[0] as f32 / PHYSICS_SCALE,
                                        p[1] as f32 / PHYSICS_SCALE,
                                    ));
                                }

                                let collider = ColliderBuilder::polyline(verts, None)
                                    .collision_groups(InteractionGroups::new(
                                        CollisionFlags::WORLD.bits().into(),
                                        CollisionFlags::RIGIDBODY.bits().into(),
                                    ))
                                    .density(0.0)
                                    .build();
                                colliders.push(collider);
                            }
                        }

                        c.set_rigidbody(Some(RigidBodyState::Inactive(
                            Box::new(rigid_body),
                            colliders,
                        )));
                    }
                } else {
                    // TODO: profile this and if it's too slow, could stagger it based on tick_time

                    let chunk_center_x =
                        c.get_chunk_x() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;
                    let chunk_center_y =
                        c.get_chunk_y() * i32::from(CHUNK_SIZE) + i32::from(CHUNK_SIZE) / 2;

                    // let dist_particle = f32::from(CHUNK_SIZE) * 0.6;
                    let dist_body = f32::from(CHUNK_SIZE) * 1.0;

                    let mut should_be_active = false;

                    // TODO: update for salva
                    // let mut psl = self.lqf_world.get_particle_system_list();
                    // while psl.is_some() && !should_be_active {
                    //     let system = psl.unwrap();
                    //     if system.get_position_buffer().iter().any(|pos| {
                    //         (pos.x * PHYSICS_SCALE as f32 - chunk_center_x as f32).abs()
                    //             < dist_particle
                    //             && (pos.y * PHYSICS_SCALE as f32 - chunk_center_y as f32).abs()
                    //                 < dist_particle
                    //     }) {
                    //         should_be_active = true;
                    //     }
                    //     psl = system.get_next();
                    // }

                    // TODO: see if using box2d's query methods instead of direct iteration is faster
                    for (_handle, rb) in self.physics.bodies.iter() {
                        if rb.body_type() == RigidBodyType::Dynamic {
                            // if body.is_awake() { // this just causes flickering
                            let pos = rb.translation();
                            let dist_x = (pos.x * PHYSICS_SCALE - chunk_center_x as f32).abs();
                            let dist_y = (pos.y * PHYSICS_SCALE - chunk_center_y as f32).abs();
                            if dist_x < dist_body && dist_y < dist_body {
                                should_be_active = true;
                            }
                            // }
                        }
                    }

                    if let Some(state) = c.get_rigidbody_mut() {
                        match state {
                            RigidBodyState::Active(h) if !should_be_active => {
                                let cls = self.physics.bodies.get(*h).unwrap().colliders().to_vec();
                                let colls = cls
                                    .iter()
                                    .map(|ch| {
                                        self.physics
                                            .colliders
                                            .remove(
                                                *ch,
                                                &mut self.physics.islands,
                                                &mut self.physics.bodies,
                                                false,
                                            )
                                            .unwrap()
                                    })
                                    .collect::<Vec<_>>();
                                let rb = self
                                    .physics
                                    .bodies
                                    .remove(
                                        *h,
                                        &mut self.physics.islands,
                                        &mut self.physics.colliders,
                                        &mut self.physics.impulse_joints,
                                        &mut self.physics.multibody_joints,
                                        true,
                                    )
                                    .unwrap();
                                *state = RigidBodyState::Inactive(Box::new(rb), colls);
                            },
                            _ => {},
                        }

                        if should_be_active && matches!(state, RigidBodyState::Inactive(_, _)) {
                            match c.get_rigidbody_mut().take().unwrap() {
                                RigidBodyState::Inactive(rb, colls) if should_be_active => {
                                    let rb_handle = self.physics.bodies.insert(*rb);
                                    for collider in colls {
                                        // let bo_handle = self
                                        //     .physics
                                        //     .fluid_pipeline
                                        //     .liquid_world
                                        //     .add_boundary(Boundary::new(Vec::new()));
                                        let _co_handle = self.physics.colliders.insert_with_parent(
                                            collider,
                                            rb_handle,
                                            &mut self.physics.bodies,
                                        );
                                        // self.physics.fluid_pipeline.coupling.register_coupling(
                                        //     bo_handle,
                                        //     co_handle,
                                        //     ColliderSampling::DynamicContactSampling,
                                        // );
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
}
