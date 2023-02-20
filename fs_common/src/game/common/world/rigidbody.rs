use glium::texture::Texture2d;
use rapier2d::{
    na::{Isometry2, Point2, Vector2},
    prelude::{
        ColliderBuilder, InteractionGroups, RigidBody, RigidBodyBuilder, RigidBodyHandle,
        SharedShape,
    },
};
// use salva2d::{integrations::rapier::ColliderSampling, object::Boundary};

use super::{
    material::MaterialInstance,
    mesh,
    physics::{Physics, PHYSICS_SCALE},
    CollisionFlags,
};

pub struct FSRigidBody {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<MaterialInstance>,
    pub body: Option<RigidBodyHandle>,
    pub image: Option<Texture2d>,
    pub image_dirty: bool,
}

impl FSRigidBody {
    pub fn from_pixels(
        pixels: Vec<MaterialInstance>,
        width: u16,
        height: u16,
    ) -> Result<Self, String> {
        if pixels.len() != width as usize * height as usize {
            return Err(format!("RigidBody::from_pixels incorrect Vec size: pixels.len() = {}, width = {width}, height = {height}", pixels.len()));
        }

        Ok(Self {
            width,
            height,
            pixels,
            body: None,
            image: None,
            image_dirty: true,
        })
    }

    pub fn from_tris(
        tris: Vec<mesh::Tri>,
        pixels: Vec<MaterialInstance>,
        width: u16,
        height: u16,
        physics: &mut Physics,
        position: (f32, f32),
    ) -> Result<Self, String> {
        if pixels.len() != width as usize * height as usize {
            return Err(format!("RigidBody::from_pixels incorrect Vec size: pixels.len() = {}, width = {width}, height = {height}", pixels.len()));
        }

        // let mut body_def = BodyDef {
        //     body_type: BodyType::DynamicBody,
        //     ..BodyDef::default()
        // };
        // body_def.position.set(position.0, position.1);

        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(Vector2::new(position.0, position.1))
            .build();
        let rb_handle = physics.bodies.insert(rigid_body);

        let mut shapes = Vec::new();
        for tri in tris {
            let verts = vec![
                (
                    tri.0 .0 as f32 / PHYSICS_SCALE,
                    tri.0 .1 as f32 / PHYSICS_SCALE,
                ),
                (
                    tri.1 .0 as f32 / PHYSICS_SCALE,
                    tri.1 .1 as f32 / PHYSICS_SCALE,
                ),
                (
                    tri.2 .0 as f32 / PHYSICS_SCALE,
                    tri.2 .1 as f32 / PHYSICS_SCALE,
                ),
            ];

            // let mut sh = PolygonShape::new();
            // sh.set(verts);

            // let mut fixture_def = FixtureDef::new(&sh);
            // fixture_def.density = 1.0;
            // fixture_def.filter.category_bits = CollisionFlags::RIGIDBODY.bits();
            // fixture_def.filter.mask_bits = CollisionFlags::all().bits();
            // bod.create_fixture(&fixture_def);

            shapes.push((
                Isometry2::new(Vector2::new(0.0, 0.0), 0.0),
                SharedShape::triangle(
                    Point2::new(verts[0].0, verts[0].1),
                    Point2::new(verts[1].0, verts[1].1),
                    Point2::new(verts[2].0, verts[2].1),
                ),
            ));
        }

        let collider = ColliderBuilder::compound(shapes)
            .collision_groups(InteractionGroups::new(
                CollisionFlags::RIGIDBODY.bits().into(),
                CollisionFlags::all().bits().into(),
            ))
            .density(1.0)
            .build();
        let _co_handle =
            physics
                .colliders
                .insert_with_parent(collider, rb_handle, &mut physics.bodies);
        // let bo_handle = physics
        //     .fluid_pipeline
        //     .liquid_world
        //     .add_boundary(Boundary::new(Vec::new()));
        // physics.fluid_pipeline.coupling.register_coupling(
        //     bo_handle,
        //     co_handle,
        //     ColliderSampling::DynamicContactSampling,
        // );

        Ok(Self {
            width,
            height,
            pixels,
            body: Some(rb_handle),
            image: None,
            image_dirty: true,
        })
    }

    pub fn get_body<'a>(&self, physics: &'a Physics) -> Option<&'a RigidBody> {
        self.body.and_then(|b| physics.bodies.get(b))
    }

    pub fn get_body_mut<'a>(&self, physics: &'a mut Physics) -> Option<&'a mut RigidBody> {
        self.body.and_then(|b| physics.bodies.get_mut(b))
    }

    pub fn make_bodies(
        pixels: &[MaterialInstance],
        width: u16,
        height: u16,
        physics: &mut Physics,
        position: (f32, f32),
    ) -> Result<Vec<FSRigidBody>, String> {
        let values = mesh::pixels_to_valuemap(pixels);
        let mesh =
            mesh::generate_mesh_only_simplified(&values, u32::from(width), u32::from(height))?;

        let loops = mesh::triangulate(&mesh);

        let mut rbs = Vec::new();

        let nearest_loop: Vec<_> = pixels
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let x = (i % width as usize) as f64;
                let y = (i / width as usize) as f64;

                let mut nearest_i = 0;
                let mut nearest_v = f64::MAX;
                for (i, a_loop) in loops.iter().enumerate() {
                    for tri in a_loop {
                        let center_x: f64 = (tri.0 .0 + tri.1 .0 + tri.2 .0) / 3.0;
                        let center_y: f64 = (tri.0 .1 + tri.1 .1 + tri.2 .1) / 3.0;

                        let dx = (center_x - x).abs();
                        let dy = (center_y - y).abs();
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq < nearest_v {
                            nearest_v = dist_sq;
                            nearest_i = i;
                        }
                    }
                }

                nearest_i
            })
            .collect();

        for (loop_i, a_loop) in loops.into_iter().enumerate() {
            let mut n_pix = 0;
            let my_pixels = pixels
                .iter()
                .cloned()
                .enumerate()
                .map(|(i, m)| {
                    if nearest_loop[i] == loop_i {
                        n_pix += 1;
                        m
                    } else {
                        MaterialInstance::air()
                    }
                })
                .collect();

            if n_pix > 0 && !a_loop.is_empty() {
                let rb =
                    FSRigidBody::from_tris(a_loop, my_pixels, width, height, physics, position)?;
                // debug!("mass = {}", rb.body.as_ref().unwrap().get_mass());
                if physics.bodies.get(rb.body.unwrap()).unwrap().mass() > 0.0 {
                    rbs.push(rb);
                }
            }
        }

        Ok(rbs)
    }

    pub fn make_body(&mut self, physics: &mut Physics, position: (f32, f32)) -> Result<(), String> {
        if self.body.is_some() {
            let b = self.body.take().unwrap();
            physics.bodies.remove(
                b,
                &mut physics.islands,
                &mut physics.colliders,
                &mut physics.impulse_joints,
                &mut physics.multibody_joints,
                true,
            );
        }

        let values = mesh::pixels_to_valuemap(&self.pixels);
        let mesh = mesh::generate_mesh_only_simplified(
            &values,
            u32::from(self.width),
            u32::from(self.height),
        )?;

        let loops = mesh::triangulate(&mesh);

        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(Vector2::new(position.0, position.1))
            .build();
        let rb_handle = physics.bodies.insert(rigid_body);

        let mut shapes = Vec::new();
        for a_loop in loops {
            for tri in a_loop {
                let verts = vec![
                    (
                        tri.0 .0 as f32 / PHYSICS_SCALE,
                        tri.0 .1 as f32 / PHYSICS_SCALE,
                    ),
                    (
                        tri.1 .0 as f32 / PHYSICS_SCALE,
                        tri.1 .1 as f32 / PHYSICS_SCALE,
                    ),
                    (
                        tri.2 .0 as f32 / PHYSICS_SCALE,
                        tri.2 .1 as f32 / PHYSICS_SCALE,
                    ),
                ];

                // let mut sh = PolygonShape::new();
                // sh.set(verts);

                // let mut fixture_def = FixtureDef::new(&sh);
                // fixture_def.density = 1.0;
                // fixture_def.filter.category_bits = CollisionFlags::RIGIDBODY.bits();
                // fixture_def.filter.mask_bits = CollisionFlags::all().bits();
                // bod.create_fixture(&fixture_def);

                shapes.push((
                    Isometry2::new(Vector2::new(0.0, 0.0), 0.0),
                    SharedShape::triangle(
                        Point2::new(verts[0].0, verts[0].1),
                        Point2::new(verts[1].0, verts[1].1),
                        Point2::new(verts[2].0, verts[2].1),
                    ),
                ));
            }
        }

        let collider = ColliderBuilder::compound(shapes).build();
        let _co_handle =
            physics
                .colliders
                .insert_with_parent(collider, rb_handle, &mut physics.bodies);
        // let bo_handle = physics
        //     .fluid_pipeline
        //     .liquid_world
        //     .add_boundary(Boundary::new(Vec::new()));
        // physics.fluid_pipeline.coupling.register_coupling(
        //     bo_handle,
        //     co_handle,
        //     ColliderSampling::DynamicContactSampling,
        // );

        self.body = Some(rb_handle);

        Ok(())
    }
}
