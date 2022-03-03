
use rapier2d::{prelude::{RigidBodyHandle, RigidBodyBuilder, ColliderBuilder, SharedShape, RigidBody, InteractionGroups}, na::{Isometry2, Vector2, Point2}};
use salva2d::{object::Boundary, integrations::rapier::ColliderSampling};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem};

use super::{material::MaterialInstance, mesh, CollisionFlags, LIQUIDFUN_SCALE, Physics};

pub struct FSRigidBody {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<MaterialInstance>,
    pub body: Option<RigidBodyHandle>,
    pub image: Option<GPUImage>,
}

impl FSRigidBody {
    pub fn from_pixels(
        pixels: Vec<MaterialInstance>,
        width: u16,
        height: u16,
    ) -> Result<Self, String> {
        if pixels.len() != width as usize * height as usize {
            return Err(format!("RigidBody::from_pixels incorrect Vec size: pixels.len() = {}, width = {}, height = {}", pixels.len(), width, height));
        }

        Ok(Self { width, height, pixels, body: None, image: None })
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
            return Err(format!("RigidBody::from_pixels incorrect Vec size: pixels.len() = {}, width = {}, height = {}", pixels.len(), width, height));
        }

        // let mut body_def = BodyDef {
        //     body_type: BodyType::DynamicBody,
        //     ..BodyDef::default()
        // };
        // body_def.position.set(position.0, position.1);

        let rigid_body = RigidBodyBuilder::new_dynamic().translation(Vector2::new(position.0, position.1)).build();
        let rb_handle = physics.bodies.insert(rigid_body);

        let mut shapes = Vec::new();
        for tri in tris {
            let verts = vec![
                (
                    tri.0 .0 as f32 / LIQUIDFUN_SCALE,
                    tri.0 .1 as f32 / LIQUIDFUN_SCALE,
                ),
                (
                    tri.1 .0 as f32 / LIQUIDFUN_SCALE,
                    tri.1 .1 as f32 / LIQUIDFUN_SCALE,
                ),
                (
                    tri.2 .0 as f32 / LIQUIDFUN_SCALE,
                    tri.2 .1 as f32 / LIQUIDFUN_SCALE,
                ),
            ];

            // let mut sh = PolygonShape::new();
            // sh.set(verts);

            // let mut fixture_def = FixtureDef::new(&sh);
            // fixture_def.density = 1.0;
            // fixture_def.filter.category_bits = CollisionFlags::RIGIDBODY.bits();
            // fixture_def.filter.mask_bits = CollisionFlags::all().bits();
            // bod.create_fixture(&fixture_def);

            shapes.push((Isometry2::new(Vector2::new(0.0, 0.0), 0.0), SharedShape::triangle(Point2::new(verts[0].0, verts[0].1), Point2::new(verts[1].0, verts[1].1), Point2::new(verts[2].0, verts[2].1))));
        }

        let collider = ColliderBuilder::compound(shapes)
            .collision_groups(InteractionGroups::new(CollisionFlags::RIGIDBODY.bits(), CollisionFlags::all().bits()))
            .density(1.0)
            .build();
        let co_handle = physics.colliders.insert_with_parent(collider, rb_handle, &mut physics.bodies);
        let bo_handle = physics.fluid_pipeline
            .liquid_world
            .add_boundary(Boundary::new(Vec::new()));
        physics.fluid_pipeline.coupling.register_coupling(
            bo_handle,
            co_handle,
            ColliderSampling::DynamicContactSampling,
        );

        Ok(Self {
            width,
            height,
            pixels,
            body: Some(rb_handle),
            image: None,
        })
    }

    pub fn get_body<'a>(&self, physics: &'a Physics) -> Option<&'a RigidBody> {
        self.body.and_then(|b| physics.bodies.get(b))
    }

    pub fn get_body_mut<'a>(&self, physics: &'a mut Physics) -> Option<&'a mut RigidBody> {
        self.body.and_then(|b| physics.bodies.get_mut(b))
    }

    pub fn update_image(&mut self) {
        let mut img = GPUSubsystem::create_image(
            self.width,
            self.height,
            sdl_gpu::sys::GPU_FormatEnum::GPU_FORMAT_RGBA,
        );
        img.set_image_filter(sdl_gpu::sys::GPU_FilterEnum::GPU_FILTER_NEAREST);

        let pixel_data: Vec<_> = self
            .pixels
            .iter()
            .flat_map(|m| vec![m.color.r, m.color.g, m.color.b, m.color.a])
            .collect();

        img.update_image_bytes(
            None as Option<GPURect>,
            &pixel_data,
            (self.width * 4).into(),
        );

        self.image = Some(img);
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
                .to_owned()
                .into_iter()
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

    pub fn make_body(
        &mut self,
        physics: &mut Physics,
        position: (f32, f32),
    ) -> Result<(), String> {
        if self.body.is_some() {
            let b = self.body.take().unwrap();
            physics.bodies.remove(b, &mut physics.islands, &mut physics.colliders, &mut physics.joints);
        }

        let values = mesh::pixels_to_valuemap(&self.pixels);
        let mesh = mesh::generate_mesh_only_simplified(
            &values,
            u32::from(self.width),
            u32::from(self.height),
        )?;

        let loops = mesh::triangulate(&mesh);

        let rigid_body = RigidBodyBuilder::new_dynamic().translation(Vector2::new(position.0, position.1)).build();
        let rb_handle = physics.bodies.insert(rigid_body);

        let mut shapes = Vec::new();
        for a_loop in loops {
            for tri in a_loop {
                let verts = vec![
                    (
                        tri.0 .0 as f32 / LIQUIDFUN_SCALE,
                        tri.0 .1 as f32 / LIQUIDFUN_SCALE,
                    ),
                    (
                        tri.1 .0 as f32 / LIQUIDFUN_SCALE,
                        tri.1 .1 as f32 / LIQUIDFUN_SCALE,
                    ),
                    (
                        tri.2 .0 as f32 / LIQUIDFUN_SCALE,
                        tri.2 .1 as f32 / LIQUIDFUN_SCALE,
                    ),
                ];

                // let mut sh = PolygonShape::new();
                // sh.set(verts);

                // let mut fixture_def = FixtureDef::new(&sh);
                // fixture_def.density = 1.0;
                // fixture_def.filter.category_bits = CollisionFlags::RIGIDBODY.bits();
                // fixture_def.filter.mask_bits = CollisionFlags::all().bits();
                // bod.create_fixture(&fixture_def);

                shapes.push((Isometry2::new(Vector2::new(0.0, 0.0), 0.0), SharedShape::triangle(Point2::new(verts[0].0, verts[0].1), Point2::new(verts[1].0, verts[1].1), Point2::new(verts[2].0, verts[2].1))));
            }
        }

        let collider = ColliderBuilder::compound(shapes).build();
        let co_handle = physics.colliders.insert_with_parent(collider, rb_handle, &mut physics.bodies);
        let bo_handle = physics.fluid_pipeline
            .liquid_world
            .add_boundary(Boundary::new(Vec::new()));
        physics.fluid_pipeline.coupling.register_coupling(
            bo_handle,
            co_handle,
            ColliderSampling::DynamicContactSampling,
        );

        self.body = Some(rb_handle);

        Ok(())
    }
}
