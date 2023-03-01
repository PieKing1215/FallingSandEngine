use std::sync::Arc;

use glium::{Blend, DrawParameters, PolygonMode};
use rapier2d::prelude::Shape;
use specs::{Join, ReadStorage, WorldExt};

use fs_common::game::common::{
    world::{
        entity::{
            GameEntity, Hitbox, PhysicsEntity, Player, PlayerGrappleState, PlayerMovementMode,
        },
        gen::structure::StructureNode,
        material::color::Color,
        particle::ParticleSystem,
        physics::PHYSICS_SCALE,
        AutoTarget, Camera, ChunkHandlerGeneric, ChunkState, Position, Velocity, World, CHUNK_SIZE,
    },
    FileHelper, Rect, Registries, Settings,
};

use crate::{
    render::{drawing::RenderTarget, rigidbody::FSRigidBodyExt},
    Client,
};

use super::{ClientChunk, ClientWorld};

pub struct WorldRenderer {
    // pub liquid_image: GPUImage,
    // pub liquid_image2: GPUImage,
    physics_dirty: bool,
}

impl WorldRenderer {
    pub fn new() -> Self {
        // let mut liquid_image =
        //     GPUSubsystem::create_image(1920 / 2, 1080 / 2, GPUFormat::GPU_FORMAT_RGBA);
        // liquid_image.set_image_filter(GPUFilter::GPU_FILTER_NEAREST);

        // let mut liquid_image2 =
        //     GPUSubsystem::create_image(1920 / 2, 1080 / 2, GPUFormat::GPU_FORMAT_RGBA);
        // liquid_image2.set_image_filter(GPUFilter::GPU_FILTER_NEAREST);

        Self {
            /*liquid_image, liquid_image2,*/ physics_dirty: false,
        }
    }

    #[allow(clippy::unused_self)]
    pub fn init(&self, _world: &mut World<ClientChunk>) {}

    #[warn(clippy::too_many_arguments)] // TODO: RenderContext struct
    #[warn(clippy::too_many_lines)]
    #[profiling::function]
    pub fn render(
        &mut self,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
        _delta_time: f64,
        settings: &Settings,
        client: &mut Client,
        partial_ticks: f64,
        registries: Arc<Registries>,
        file_helper: &FileHelper,
    ) {
        // TODO
        // if world.lqf_world.get_debug_draw().is_none() {
        //     self.init(world);
        // }

        // draw world

        let (position_storage, velocity_storage, camera_storage) = world.ecs.system_data::<(
            ReadStorage<Position>,
            ReadStorage<Velocity>,
            ReadStorage<Camera>,
        )>();

        let camera_pos = (&position_storage, velocity_storage.maybe(), &camera_storage)
            .join()
            .map(|(p, v, _c)| Position {
                x: p.x + v.map_or(0.0, |v| v.x) * partial_ticks,
                y: p.y + v.map_or(0.0, |v| v.y) * partial_ticks,
            })
            .next()
            .expect("No Camera in world!");

        let loader_pos = match client {
            Client { world: Some(ClientWorld { local_entity }), .. } => local_entity
                .and_then(|local| position_storage.get(local))
                .or(Some(&camera_pos))
                .map(|pos| (pos.x, pos.y))
                .unwrap(),
            _ => (camera_pos.x, camera_pos.y),
        };

        drop(position_storage);
        drop(velocity_storage);
        drop(camera_storage);

        let camera_scale = client.camera_scale;

        target.transform.push();
        target.transform.translate(
            f64::from(target.width()) / 2.0,
            f64::from(target.height()) / 2.0,
        );
        target.transform.scale(camera_scale, camera_scale);
        target.transform.translate(-camera_pos.x, -camera_pos.y);

        let screen_zone = world
            .chunk_handler
            .get_screen_zone((camera_pos.x, camera_pos.y)); // note we always use the camera for the screen zone
        let active_zone = world.chunk_handler.get_active_zone(loader_pos);
        let load_zone = world.chunk_handler.get_load_zone(loader_pos);
        let unload_zone = world.chunk_handler.get_unload_zone(loader_pos);

        // let clip = canvas.clip_rect();
        // if game.settings.cull_chunks {
        //     canvas.set_clip_rect(target.transform.transform_rect(screen_zone));
        // }

        let chunk_tex_data = {
            profiling::scope!("chunks2");
            let texs = world
                .chunk_handler
                .loaded_chunks
                .iter_mut()
                .filter_map(|(_i, ch)| {
                    let rc = Rect::new_wh(
                        ch.chunk_x * i32::from(CHUNK_SIZE),
                        ch.chunk_y * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );

                    target.transform.push();
                    target.transform.translate(
                        ch.chunk_x * i32::from(CHUNK_SIZE),
                        ch.chunk_y * i32::from(CHUNK_SIZE),
                    );

                    if (settings.debug && !settings.cull_chunks) || rc.intersects(&screen_zone) {
                        ch.prep_render(target, settings, file_helper);

                        target.transform.pop();

                        // ch.render(target, settings);
                        // ch.graphics.texture
                        // let image = glium::texture::RawImage2d::from_raw_rgba((&ch.graphics.pixel_data).to_vec(), (CHUNK_SIZE.into(), CHUNK_SIZE.into()));
                        // Some(((ch.chunk_x as f32 * f32::from(CHUNK_SIZE), ch.chunk_y as f32 * f32::from(CHUNK_SIZE)), image))
                        ch.graphics.data.as_ref().map(|t| {
                            (
                                (
                                    ch.chunk_x as f32 * f32::from(CHUNK_SIZE),
                                    ch.chunk_y as f32 * f32::from(CHUNK_SIZE),
                                ),
                                t.clone(),
                            )
                        })
                    } else {
                        target.transform.pop();
                        None
                    }
                })
                .collect::<Vec<_>>();

            target.draw_chunks_2(&texs);

            // if !texs.0.is_empty() {
            //     profiling::scope!("draw");
            //     let ta = {
            //         profiling::scope!("Texture2dArray::with_mipmaps");
            //         Texture2dArray::with_mipmaps(&target.display, texs.1, MipmapsOption::NoMipmap).unwrap()
            //     };
            //     target.draw_chunks(&texs.0, &ta);
            // }
            texs
        };

        {
            profiling::scope!("chunks overlay");
            let mut structure_lines = vec![];
            world
                .chunk_handler
                .loaded_chunks
                .iter_mut()
                .for_each(|(_i, ch)| {
                    let rc = Rect::new_wh(
                        ch.chunk_x * i32::from(CHUNK_SIZE),
                        ch.chunk_y * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );

                    if let (true, Some(set)) = (settings.debug, settings.draw_structure_set.clone())
                    {
                        if let Some(v) = registries.structure_sets.get(&set) {
                            let (start_x, start_y) =
                                v.nearest_start_chunk((ch.chunk_x, ch.chunk_y), world.seed as _);
                            let should_gen_start = v.should_generate_at(
                                (start_x, start_y),
                                world.seed as _,
                                &registries,
                                true,
                            );
                            structure_lines.push((
                                (
                                    (ch.chunk_x * i32::from(CHUNK_SIZE)) as f32,
                                    (ch.chunk_y * i32::from(CHUNK_SIZE)) as f32,
                                ),
                                (
                                    (start_x * i32::from(CHUNK_SIZE)) as f32,
                                    (start_y * i32::from(CHUNK_SIZE)) as f32,
                                ),
                                if start_x == ch.chunk_x && start_y == ch.chunk_y {
                                    Color::GREEN
                                } else if should_gen_start {
                                    Color::ORANGE.with_a(0.25)
                                } else {
                                    Color::RED.with_a(0.125)
                                },
                            ));
                        }
                    }

                    target.transform.push();
                    target.transform.translate(
                        ch.chunk_x * i32::from(CHUNK_SIZE),
                        ch.chunk_y * i32::from(CHUNK_SIZE),
                    );

                    if (settings.debug && !settings.cull_chunks) || rc.intersects(&screen_zone) {
                        ch.render(target, settings);

                        if settings.debug && settings.draw_chunk_dirty_rects {
                            if let Some(dr) = ch.dirty_rect {
                                let rect = dr.into_f32();
                                target.rectangle(
                                    rect,
                                    Color::rgba(255, 64, 64, 127),
                                    DrawParameters {
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                                target.rectangle(
                                    rect,
                                    Color::rgba(255, 64, 64, 127),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(1.0),
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                            }
                            if ch.graphics.was_dirty {
                                let rect = Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE).into_f32();
                                target.rectangle(
                                    rect,
                                    Color::rgba(255, 255, 64, 127),
                                    DrawParameters {
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                                target.rectangle(
                                    rect,
                                    Color::rgba(255, 255, 64, 127),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(1.0),
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                    }

                    if settings.debug && settings.draw_chunk_state_overlay {
                        let rect = Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE);

                        let alpha: u8 = (settings.draw_chunk_state_overlay_alpha * 255.0) as u8;
                        let color = match ch.state {
                            ChunkState::NotGenerated => Color::rgba(127, 127, 127, alpha),
                            ChunkState::Generating(stage) => Color::rgba(
                                64,
                                (f32::from(stage)
                                    / f32::from(world.chunk_handler.generator.max_gen_stage())
                                    * 255.0) as u8,
                                255,
                                alpha,
                            ),
                            ChunkState::Cached => Color::rgba(255, 127, 64, alpha),
                            ChunkState::Active => Color::rgba(64, 255, 64, alpha),
                        };
                        target.rectangle(
                            rect.into_f32(),
                            color,
                            DrawParameters {
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                        target.rectangle(
                            rect.into_f32(),
                            color,
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );

                        // let ind = world.chunk_handler.chunk_index(ch.chunk_x, ch.chunk_y);
                        // let ind = world.chunk_handler.chunk_update_order(ch.chunk_x, ch.chunk_y);
                        // let tex = canvas.texture_creator();
                        // let txt_sf = fonts.pixel_operator
                        //     .render(format!("{}", ind).as_str())
                        //     .solid(Color::RGB(255, 255, 255)).unwrap();
                        // let txt_tex = tex.create_texture_from_surface(&txt_sf).unwrap();

                        // let aspect = txt_sf.width() as f32 / txt_sf.height() as f32;
                        // let mut txt_height = rect.height() as f32 * 0.75;
                        // let mut txt_width = (aspect * txt_height as f32) as u32;

                        // let max_width = (rect.w as f32 * 0.9) as u32;

                        // if txt_width > max_width as u32 {
                        //     txt_width = max_width as u32;
                        //     txt_height = 1.0 / aspect * txt_width as f32;
                        // }

                        // let txt_rec = Rect::new(rect.x + rect.w/2 - (txt_width as i32)/2, rect.y, txt_width, txt_height as u32);
                        // canvas.copy(&txt_tex, None, Some(txt_rec)).unwrap();
                    }

                    target.transform.pop();
                });

            target.lines(
                structure_lines,
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
        }

        // draw liquids

        if self.physics_dirty {
            self.physics_dirty = false;

            // let mut liquid_target = self.liquid_image.get_target();
            // liquid_target.clear();

            // for (_handle, fluid) in world.physics.fluid_pipeline.liquid_world.fluids().iter() {
            //     for (_idx, particle) in fluid.positions.iter().enumerate() {
            //         let (x, y) = target.transform.transform((
            //             particle.coords[0] * PHYSICS_SCALE,
            //             particle.coords[1] * PHYSICS_SCALE,
            //         ));
            //         // target.circle_filled(x as f32, y as f32, 2.0, Color::CYAN.into_sdl());
            //     }
            // }

            // if let Some(particle_system) = world.lqf_world.get_particle_system_list() {
            //     let particle_count = particle_system.get_particle_count();
            //     let particle_colors: &[b2ParticleColor] = particle_system.get_color_buffer();
            //     let particle_positions: &[Vec2] = particle_system.get_position_buffer();

            //     for i in 0..particle_count as usize {
            //         let pos = particle_positions[i];
            //         let color = particle_colors[i];
            //         let cam_x = camera_pos.x.floor();
            //         let cam_y = camera_pos.y.floor();
            //         GPUSubsystem::set_shape_blend_mode(
            //             sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_SET,
            //         );
            //         let color = Color::rgba(color.r, color.g, color.b, color.a);
            //         // let color = Color::rgba(64, 90, 255, 191);
            //         liquid_target.pixel(
            //             pos.x * PHYSICS_SCALE - cam_x as f32 + 1920.0 / 4.0 - 1.0,
            //             pos.y * PHYSICS_SCALE - cam_y as f32 + 1080.0 / 4.0 - 1.0,
            //             color,
            //         );
            //         // liquid_target.circle_filled(pos.x * 2.0 - camera_pos.x as f32 + 1920.0/4.0, pos.y * 2.0 - camera_pos.y as f32 + 1080.0/4.0, 2.0, Color::RGB(100, 100, 255));
            //     }

            //     GPUSubsystem::set_shape_blend_mode(
            //         sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL,
            //     );

            //     let mut liquid_target2 = self.liquid_image2.get_target();
            //     liquid_target2.clear();

            //     self.liquid_image
            //         .set_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_SET);

            //     shaders.liquid_shader.activate();
            //     self.liquid_image
            //         .blit_rect(None::<GPURect>, &mut liquid_target2, None);
            //     Shader::deactivate();

            //     self.liquid_image
            //         .set_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);
            // };
        }

        // TODO: transforming screen zone here is not the right way to do this, it causes some jumping when x or y switch between + and -
        // self.liquid_image2.blit_rect(
        //     None,
        //     target,
        //     Some(target.transform.transform_rect(screen_zone).into_sdl()),
        // );

        // draw solids

        {
            profiling::scope!("rigidbodies");
            target.transform.push();
            target.transform.scale(PHYSICS_SCALE, PHYSICS_SCALE);
            for rb in &mut world.rigidbodies {
                rb.update_image(target);

                if let Some(body) = rb.get_body(&world.physics) {
                    if let Some(img) = &rb.image {
                        let (rx, ry) = (
                            body.position().translation.vector[0],
                            body.position().translation.vector[1],
                        );

                        target.transform.push();
                        target.transform.translate(rx, ry);
                        target.transform.rotate(body.rotation().angle());

                        target.draw_texture_flipped(
                            Rect::new_wh(
                                0.0,
                                0.0,
                                rb.width as f32 / PHYSICS_SCALE,
                                rb.height as f32 / PHYSICS_SCALE,
                            ),
                            img,
                            DrawParameters {
                                blend: Blend::alpha_blending(),
                                ..DrawParameters::default()
                            },
                        );

                        target.transform.pop();
                    }
                }
            }
            target.transform.pop();
        }

        // physics debug draw

        if settings.debug && settings.physics_dbg_draw {
            profiling::scope!("physics debug");
            target.transform.push();
            target.transform.scale(PHYSICS_SCALE, PHYSICS_SCALE);

            fn draw_shape(
                shape: &dyn Shape,
                x: f32,
                y: f32,
                angle: f32,
                target: &mut RenderTarget,
                color: Color,
            ) {
                target.transform.push();
                target.transform.translate(x, y);
                target.transform.rotate(angle);
                if let Some(comp) = shape.as_compound() {
                    for (_iso, shape) in comp.shapes() {
                        draw_shape(&**shape, 0.0, 0.0, 0.0, target, color);
                    }
                } else if let Some(cuboid) = shape.as_cuboid() {
                    let (x1, y1) = (-cuboid.half_extents[0], -cuboid.half_extents[1]);
                    let (x2, y2) = (cuboid.half_extents[0], cuboid.half_extents[1]);
                    target.rectangle(
                        Rect::new(x1, y1, x2, y2),
                        color,
                        DrawParameters {
                            polygon_mode: PolygonMode::Line,
                            line_width: Some(1.0),
                            blend: Blend::alpha_blending(),
                            ..Default::default()
                        },
                    );
                } else if let Some(polyline) = shape.as_polyline() {
                    for seg in polyline.segments() {
                        let (x1, y1) = (seg.a[0], seg.a[1]);
                        let (x2, y2) = (seg.b[0], seg.b[1]);
                        target.line(
                            (x1, y1),
                            (x2, y2),
                            color,
                            DrawParameters {
                                blend: Blend::alpha_blending(),
                                ..DrawParameters::default()
                            },
                        );
                    }
                } else if let Some(_poly) = shape.as_convex_polygon() {
                    // target.polygon(
                    //     poly.points()
                    //         .iter()
                    //         .flat_map(|v| {
                    //             let (x, y) = target.transform.transform((v[0], v[1]));
                    //             [x as f32, y as f32]
                    //         })
                    //         .collect(),
                    //     color.into_sdl(),
                    // );
                } else if let Some(trimesh) = shape.as_trimesh() {
                    for tri in trimesh.triangles() {
                        let (x1, y1) = (tri.a[0], tri.a[1]);
                        let (x2, y2) = (tri.b[0], tri.b[1]);
                        let (x3, y3) = (tri.c[0], tri.c[1]);
                        target.triangle(
                            (x1, y1),
                            (x2, y2),
                            (x3, y3),
                            color,
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                    }
                } else if let Some(tri) = shape.as_triangle() {
                    let (x1, y1) = (x + tri.a[0], y + tri.a[1]);
                    let (x2, y2) = (x + tri.b[0], y + tri.b[1]);
                    let (x3, y3) = (x + tri.c[0], y + tri.c[1]);
                    target.triangle(
                        (x1, y1),
                        (x2, y2),
                        (x3, y3),
                        color,
                        DrawParameters {
                            polygon_mode: PolygonMode::Line,
                            line_width: Some(1.0),
                            blend: Blend::alpha_blending(),
                            ..Default::default()
                        },
                    );
                }
                target.transform.pop();
            }

            // TODO: physics_dbg_draw_joint
            // TODO: physics_dbg_draw_pair
            // TODO: physics_dbg_draw_particle

            for (_handle, b) in world.physics.bodies.iter() {
                let (rx, ry) = (
                    b.position().translation.vector[0],
                    b.position().translation.vector[1],
                );

                target.transform.push();
                target.transform.translate(rx, ry);
                target.transform.rotate(b.rotation().angle());
                // target.line((0.0, 0.0), (0.5, 0.0), Color::BLUE, DrawParameters::default());
                // target.line((0.0, 0.0), (0.0, 0.5), Color::RED, DrawParameters::default());
                // target.circle(x as f32, y as f32, 3.0, Color::GREEN.into_sdl());

                if settings.physics_dbg_draw_center_of_mass {
                    let com = b.mass_properties().local_mprops.local_com;
                    target.line(
                        (com.x, com.y),
                        (com.x + 0.5, com.y),
                        Color::BLUE,
                        DrawParameters::default(),
                    );
                    target.line(
                        (com.x, com.y),
                        (com.x, com.y + 0.5),
                        Color::RED,
                        DrawParameters::default(),
                    );
                    // target.circle(x as f32, y as f32, 2.0, Color::RED.into_sdl());
                }
                target.transform.pop();

                for c in b.colliders() {
                    let col = world.physics.colliders.get(*c).unwrap();

                    if settings.physics_dbg_draw_shape {
                        let shape = col.shape();
                        draw_shape(
                            shape,
                            rx,
                            ry,
                            b.rotation().angle(),
                            target,
                            Color::rgba(
                                0x00,
                                0xff,
                                0x00,
                                if b.is_sleeping() { 0x64 } else { 0xff },
                            ),
                        );
                    }

                    if settings.physics_dbg_draw_aabb {
                        let aabb = col.compute_aabb();

                        target.transform.push();
                        target.transform.translate(aabb.center().x, aabb.center().y);

                        let (x1, y1) = (-aabb.half_extents()[0], -aabb.half_extents()[1]);
                        let (x2, y2) = (aabb.half_extents()[0], aabb.half_extents()[1]);

                        target.rectangle(
                            Rect::new(x1, y1, x2, y2),
                            Color::rgba(0xff, 0, 0xff, if b.is_sleeping() { 0x64 } else { 0xff }),
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );

                        target.transform.pop();
                    }
                }
            }

            target.transform.pop();
        }

        {
            profiling::scope!("particles");
            let particle_system = world.ecs.read_resource::<ParticleSystem>();
            target.draw_particles(&particle_system.active, partial_ticks as f32);
        }

        if settings.draw_lighting {
            target.draw_chunks_light(
                &chunk_tex_data,
                (camera_pos.x as f32, camera_pos.y as f32),
                settings.lighting_smooth,
                settings.lighting_overlay,
                settings.lighting_linear_blend,
            );
        }

        {
            profiling::scope!("ecs debug");

            let (
                game_entity_storage,
                position_storage,
                velocity_storage,
                physics_storage,
                node_storage,
            ) = world.ecs.system_data::<(
                ReadStorage<GameEntity>,
                ReadStorage<Position>,
                ReadStorage<Velocity>,
                ReadStorage<PhysicsEntity>,
                ReadStorage<StructureNode>,
            )>();

            if settings.debug && settings.draw_structure_bounds {
                let mut snode_rects_1 = vec![];
                let mut snode_rects_2 = vec![];
                (&position_storage, &node_storage)
                    .join()
                    .for_each(|(pos, node)| {
                        target.transform.push();
                        target.transform.translate(pos.x, pos.y);

                        let (x1, y1) = (
                            -((node.depth + 1) as f64 * 3.0) + pos.x,
                            -((node.depth + 1) as f64 * 3.0) + pos.y,
                        );
                        let (x2, y2) = (
                            ((node.depth + 1) as f64 * 3.0) + pos.x,
                            ((node.depth + 1) as f64 * 3.0) + pos.y,
                        );

                        let alpha = if node.generated.is_some() { 80 } else { 255 };

                        snode_rects_1.push((
                            Rect::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32),
                            Color::rgba(64, 64, 255, alpha),
                        ));

                        target.transform.pop();

                        if let Some(Ok(gen)) = &node.generated {
                            snode_rects_2.push((
                                Rect::new(
                                    gen.bounds.x1 as f32,
                                    gen.bounds.y1 as f32,
                                    gen.bounds.x2 as f32,
                                    gen.bounds.y2 as f32,
                                ),
                                Color::rgba(64, 255, 255, alpha),
                            ));
                        }
                    });

                target.rectangles_colored(
                    &snode_rects_2,
                    DrawParameters {
                        polygon_mode: PolygonMode::Fill,
                        line_width: Some(1.0),
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                );

                target.rectangles_colored(
                    &snode_rects_1,
                    DrawParameters {
                        polygon_mode: PolygonMode::Fill,
                        line_width: Some(1.0),
                        blend: Blend::alpha_blending(),
                        ..Default::default()
                    },
                );
            }

            (
                &game_entity_storage,
                &position_storage,
                velocity_storage.maybe(),
                physics_storage.maybe(),
            )
                .join()
                .for_each(
                    |(_ge, pos, vel, _phys): (
                        &GameEntity,
                        &Position,
                        Option<&Velocity>,
                        Option<&PhysicsEntity>,
                    )| {
                        let mut draw = |x: f64, y: f64, alpha: u8| {
                            target.transform.push();
                            target.transform.translate(x, y);

                            let (x1, y1) = (-1.0, -1.0);
                            let (x2, y2) = (1.0, 1.0);

                            target.rectangle(
                                Rect::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32),
                                Color::rgba(64, 255, 64, alpha),
                                DrawParameters {
                                    polygon_mode: PolygonMode::Line,
                                    line_width: Some(1.0),
                                    blend: Blend::alpha_blending(),
                                    ..Default::default()
                                },
                            );

                            if let Some(vel) = vel {
                                target.line(
                                    (0.0, 0.0),
                                    (vel.x, vel.y),
                                    Color::rgba(64, 255, 64, alpha),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(1.0),
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                            }

                            target.transform.pop();
                        };

                        let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * partial_ticks;
                        let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * partial_ticks;
                        draw(lerp_x, lerp_y, 255);
                        draw(pos.x, pos.y, 80);
                    },
                );

            let (position_storage, hitbox_storage, velocity_storage) = world.ecs.system_data::<(
                ReadStorage<Position>,
                ReadStorage<Hitbox>,
                ReadStorage<Velocity>,
            )>();

            (&position_storage, &hitbox_storage, velocity_storage.maybe())
                .join()
                .for_each(|(pos, hit, vel)| {
                    let mut draw = |x: f64, y: f64, alpha: u8| {
                        target.transform.push();
                        target.transform.translate(x, y);

                        let (x1, y1) = (f64::from(hit.x1), f64::from(hit.y1));
                        let (x2, y2) = (f64::from(hit.x2), f64::from(hit.y2));

                        target.rectangle(
                            Rect::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32),
                            Color::rgba(255, 64, 64, alpha),
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );

                        target.transform.pop();
                    };

                    let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * partial_ticks;
                    let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * partial_ticks;
                    draw(lerp_x, lerp_y, 255);
                    draw(pos.x, pos.y, 80);
                });

            let (position_storage, velocity_storage, target_storage) = world.ecs.system_data::<(
                ReadStorage<Position>,
                ReadStorage<Velocity>,
                ReadStorage<AutoTarget>,
            )>();

            (&position_storage, velocity_storage.maybe(), &target_storage)
                .join()
                .for_each(|(pos, vel, at)| {
                    let mut draw = |x: f64, y: f64, alpha: u8| {
                        target.transform.push();
                        target.transform.translate(x, y);

                        let (x1, y1) = (-1.0, -1.0);
                        let (x2, y2) = (1.0, 1.0);

                        target.rectangle(
                            Rect::new(x1 as f32, y1 as f32, x2 as f32, y2 as f32),
                            Color::rgba(64, 255, 64, alpha),
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );

                        let target_pos = at.get_target_pos(&position_storage);
                        if let Some(target_pos) = target_pos {
                            let (line_x1, line_y1) = (0.0, 0.0);
                            let (line_x2, line_y2) = (target_pos.x - x, target_pos.y - y);

                            target.line(
                                (line_x1 as f32, line_y1 as f32),
                                (line_x2 as f32, line_y2 as f32),
                                Color::rgba(255, 255, 64, alpha / 2),
                                DrawParameters {
                                    polygon_mode: PolygonMode::Line,
                                    line_width: Some(1.0),
                                    blend: Blend::alpha_blending(),
                                    ..Default::default()
                                },
                            );
                        }

                        target.transform.pop();
                    };

                    let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * partial_ticks;
                    let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * partial_ticks;
                    draw(lerp_x, lerp_y, 255);
                    draw(pos.x, pos.y, 80);
                });

            let (entities, position_storage, velocity_storage, player_storage) =
                world.ecs.system_data::<(
                    specs::Entities,
                    ReadStorage<Position>,
                    ReadStorage<Velocity>,
                    ReadStorage<Player>,
                )>();

            (&entities, &player_storage)
                .join()
                .for_each(|(ent, player)| match &player.movement {
                    PlayerMovementMode::Normal { grapple_state, .. } => {
                        let mut draw_grapple = |grapple: &specs::Entity, pivots: &Vec<Position>| {
                            let player_pos = position_storage
                                .get(ent)
                                .expect("Missing Position on Player");
                            let grapple_pos = position_storage
                                .get(*grapple)
                                .expect("Missing Position on grapple");
                            let player_vel = velocity_storage
                                .get(ent)
                                .expect("Missing Velocity on Player");
                            let grapple_vel = velocity_storage
                                .get(*grapple)
                                .expect("Missing Velocity on grapple");

                            // target.set_line_thickness(2.0);
                            if pivots.is_empty() {
                                let (x1, y1) = (
                                    player_pos.x + player_vel.x * partial_ticks,
                                    player_pos.y + player_vel.y * partial_ticks,
                                );
                                let (x2, y2) = (
                                    grapple_pos.x + grapple_vel.x * partial_ticks,
                                    grapple_pos.y + grapple_vel.y * partial_ticks,
                                );

                                target.line(
                                    (x1 as f32, y1 as f32),
                                    (x2 as f32, y2 as f32),
                                    Color::rgba(191, 191, 191, 255),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(client.camera_scale as f32),
                                        blend: Blend::alpha_blending(),
                                        ..Default::default()
                                    },
                                );
                            } else {
                                {
                                    let (x1, y1) = (
                                        grapple_pos.x + grapple_vel.x * partial_ticks,
                                        grapple_pos.y + grapple_vel.y * partial_ticks,
                                    );
                                    let (x2, y2) = (pivots[0].x, pivots[0].y);
                                    target.line(
                                        (x1 as f32, y1 as f32),
                                        (x2 as f32, y2 as f32),
                                        Color::rgba(191, 191, 191, 255),
                                        DrawParameters {
                                            polygon_mode: PolygonMode::Line,
                                            line_width: Some(client.camera_scale as f32),
                                            blend: Blend::alpha_blending(),
                                            ..Default::default()
                                        },
                                    );
                                }

                                if pivots.len() > 1 {
                                    for i in 1..pivots.len() {
                                        let p1 = &pivots[i - 1];
                                        let p2 = &pivots[i];
                                        let (x1, y1) = (p1.x, p1.y);
                                        let (x2, y2) = (p2.x, p2.y);

                                        target.line(
                                            (x1 as f32, y1 as f32),
                                            (x2 as f32, y2 as f32),
                                            Color::rgba(191, 191, 191, 255),
                                            DrawParameters {
                                                polygon_mode: PolygonMode::Line,
                                                line_width: Some(client.camera_scale as f32),
                                                blend: Blend::alpha_blending(),
                                                ..Default::default()
                                            },
                                        );
                                    }
                                }

                                {
                                    let (x1, y1) =
                                        (pivots[pivots.len() - 1].x, pivots[pivots.len() - 1].y);
                                    let (x2, y2) = (
                                        player_pos.x + player_vel.x * partial_ticks,
                                        player_pos.y + player_vel.y * partial_ticks,
                                    );
                                    target.line(
                                        (x1 as f32, y1 as f32),
                                        (x2 as f32, y2 as f32),
                                        Color::rgba(191, 191, 191, 255),
                                        DrawParameters {
                                            polygon_mode: PolygonMode::Line,
                                            line_width: Some(client.camera_scale as f32),
                                            blend: Blend::alpha_blending(),
                                            ..Default::default()
                                        },
                                    );
                                }
                            }
                            // target.set_line_thickness(1.0);
                        };

                        match grapple_state {
                            PlayerGrappleState::Out { entity, pivots, .. } => {
                                draw_grapple(entity, pivots);
                            },
                            PlayerGrappleState::Cancelled { entity } => {
                                draw_grapple(entity, &vec![]);
                            },
                            PlayerGrappleState::Ready | PlayerGrappleState::Used => (),
                        }
                    },
                    PlayerMovementMode::Free => (),
                });
        }
        // canvas.set_clip_rect(clip);

        if settings.debug && settings.draw_chunk_grid {
            for x in -10..10 {
                for y in -10..10 {
                    let rc_x = x + (camera_pos.x / f64::from(CHUNK_SIZE)) as i32;
                    let rc_y = y + (camera_pos.y / f64::from(CHUNK_SIZE)) as i32;
                    let rc = Rect::new_wh(
                        rc_x * i32::from(CHUNK_SIZE),
                        rc_y * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );

                    target.rectangle(
                        rc.into_f32(),
                        Color::rgba(64, 64, 64, 127),
                        DrawParameters {
                            polygon_mode: PolygonMode::Line,
                            line_width: Some(1.0),
                            blend: Blend::alpha_blending(),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        if settings.debug && settings.draw_origin {
            let len: f32 = 16.0;
            target.rectangle(
                Rect::new_wh(-len - 2.0, -1.0, len * 2.0 + 4.0, 3.0),
                Color::rgba(0, 0, 0, 127),
                DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangle(
                Rect::new_wh(-1.0, -len - 2.0, 3.0, len * 2.0 + 4.0),
                Color::rgba(0, 0, 0, 127),
                DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangle(
                Rect::new_wh(0.0, 0.0, 2.0, 2.0),
                Color::WHITE,
                DrawParameters::default(),
            );

            // target.line(
            //     origin.0 as f32 - len,
            //     origin.1 as f32,
            //     origin.0 as f32 + len,
            //     origin.1 as f32,
            //     Color::rgba(255, 0, 0, 255).into_sdl(),
            // );
            // target.line(
            //     origin.0 as f32,
            //     origin.1 as f32 - len,
            //     origin.0 as f32,
            //     origin.1 as f32 + len,
            //     Color::rgba(0, 255, 0, 255).into_sdl(),
            // );
        }

        if settings.debug && settings.draw_load_zones {
            target.rectangle(
                unload_zone.into_f32(),
                Color::rgba(255, 0, 0, 127),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangle(
                load_zone.into_f32(),
                Color::rgba(255, 127, 0, 127),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangle(
                active_zone.into_f32(),
                Color::rgba(255, 255, 0, 127),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangle(
                screen_zone.into_f32(),
                Color::rgba(0, 255, 0, 127),
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
        }

        target.transform.pop();

        // draw overlay
    }

    pub fn mark_liquid_dirty(&mut self) {
        self.physics_dirty = true;
    }
}

impl Default for WorldRenderer {
    fn default() -> Self {
        Self::new()
    }
}
