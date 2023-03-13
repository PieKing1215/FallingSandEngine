use std::sync::Arc;

use chunksystem::ChunkQuery;
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
        AutoTarget, Camera, Chunk, ChunkState, Position, SidedChunk, Velocity, World, CHUNK_SIZE,
    },
    FileHelper, Rect, Registries, Settings,
};

use crate::{
    render::{drawing::RenderTarget, rigidbody::FSRigidBodyExt},
    Client,
};

use super::{chunk_data::tile_entity::ClientTileEntityExt, ClientChunk, ClientWorld};

pub struct WorldRenderer {
    physics_dirty: bool,
}

impl WorldRenderer {
    pub fn new() -> Self {
        Self { physics_dirty: false }
    }

    #[allow(clippy::unused_self)]
    pub fn init(&self, _world: &mut World<ClientChunk>) {}

    #[profiling::function]
    pub fn render(
        &mut self,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
        ctx: RenderContext,
    ) {
        // draw world

        let (position_storage, velocity_storage, camera_storage) = world.ecs.system_data::<(
            ReadStorage<Position>,
            ReadStorage<Velocity>,
            ReadStorage<Camera>,
        )>();

        let camera_pos = (&position_storage, velocity_storage.maybe(), &camera_storage)
            .join()
            .map(|(p, v, _c)| Position {
                x: p.x + v.map_or(0.0, |v| v.x) * ctx.partial_ticks,
                y: p.y + v.map_or(0.0, |v| v.y) * ctx.partial_ticks,
            })
            .next()
            .expect("No Camera in world!");

        let loader_pos = match ctx.client {
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

        let camera_scale = ctx.client.camera_scale;

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

        let chunk_tex_data = {
            profiling::scope!("build chunk_tex_data");
            unsafe { world.chunk_handler.manager.raw_mut().iter_mut() }
                .filter_map(|(_i, ch)| {
                    let rc = Rect::new_wh(
                        ch.chunk_x() * i32::from(CHUNK_SIZE),
                        ch.chunk_y() * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );

                    if (ctx.settings.debug && !ctx.settings.cull_chunks)
                        || rc.intersects(&screen_zone)
                    {
                        target.transform.push();
                        target.transform.translate(
                            ch.chunk_x() * i32::from(CHUNK_SIZE),
                            ch.chunk_y() * i32::from(CHUNK_SIZE),
                        );

                        ch.prep_render(target, ctx.settings, ctx.file_helper);

                        target.transform.pop();

                        // ch.render(target, settings);
                        // ch.graphics.texture
                        // let image = glium::texture::RawImage2d::from_raw_rgba((&ch.graphics.pixel_data).to_vec(), (CHUNK_SIZE.into(), CHUNK_SIZE.into()));
                        // Some(((ch.chunk_x as f32 * f32::from(CHUNK_SIZE), ch.chunk_y as f32 * f32::from(CHUNK_SIZE)), image))
                        ch.graphics.data.as_ref().map(|t| {
                            (
                                (
                                    ch.chunk_x() as f32 * f32::from(CHUNK_SIZE),
                                    ch.chunk_y() as f32 * f32::from(CHUNK_SIZE),
                                ),
                                t.clone(),
                            )
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        target.draw_chunks(&chunk_tex_data);

        // draw liquids
        // TODO: remove
        if self.physics_dirty {
            self.physics_dirty = false;
        }

        for ch in world.chunk_handler.manager.chunks_iter_mut() {
            for te in ch.sided_tile_entities_mut() {
                te.render(target);
            }
        }

        self.draw_rigidbodies(world, target);

        // draw physics debug
        if ctx.settings.debug && ctx.settings.physics_dbg_draw {
            self.draw_physics_debug(world, target, &ctx);
        }

        // draw particles
        {
            profiling::scope!("particles");
            let particle_system = world.ecs.read_resource::<ParticleSystem>();
            target.draw_particles(&particle_system.active, ctx.partial_ticks as f32);
        }

        // multiply lighting
        if ctx.settings.draw_lighting {
            target.draw_chunks_light(
                &chunk_tex_data,
                (camera_pos.x as f32, camera_pos.y as f32),
                ctx.settings.lighting_smooth,
                ctx.settings.lighting_dithering,
                ctx.settings.lighting_overlay,
                ctx.settings.lighting_linear_blend,
            );
        }

        // overlays

        self.draw_chunk_overlays(&screen_zone, world, target, &ctx);

        self.draw_ecs_debug(world, target, &ctx);

        if ctx.settings.debug && ctx.settings.draw_chunk_grid {
            self.draw_chunk_grid(&camera_pos, target);
        }

        if ctx.settings.debug && ctx.settings.draw_origin {
            self.draw_origin(target);
        }

        if ctx.settings.debug && ctx.settings.draw_load_zones {
            self.draw_load_zones(loader_pos, Some(camera_pos.into()), world, target);
        }

        target.transform.pop();
    }

    fn draw_ecs_debug(
        &mut self,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
        ctx: &RenderContext,
    ) {
        profiling::scope!("draw_ecs_debug");

        let (
            entities,
            game_entity_storage,
            position_storage,
            velocity_storage,
            physics_storage,
            node_storage,
            hitbox_storage,
            target_storage,
            player_storage,
        ) = world.ecs.system_data::<(
            specs::Entities,
            ReadStorage<GameEntity>,
            ReadStorage<Position>,
            ReadStorage<Velocity>,
            ReadStorage<PhysicsEntity>,
            ReadStorage<StructureNode>,
            ReadStorage<Hitbox>,
            ReadStorage<AutoTarget>,
            ReadStorage<Player>,
        )>();

        // draw structure bounds
        if ctx.settings.debug && ctx.settings.draw_structure_bounds {
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

        // draw entity positions
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

                    let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * ctx.partial_ticks;
                    let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * ctx.partial_ticks;
                    draw(lerp_x, lerp_y, 255);
                    draw(pos.x, pos.y, 80);
                },
            );

        // draw entity hitboxes
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

                let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * ctx.partial_ticks;
                let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * ctx.partial_ticks;
                draw(lerp_x, lerp_y, 255);
                draw(pos.x, pos.y, 80);
            });

        // draw entity targets
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

                let lerp_x = pos.x + vel.map_or(0.0, |v| v.x) * ctx.partial_ticks;
                let lerp_y = pos.y + vel.map_or(0.0, |v| v.y) * ctx.partial_ticks;
                draw(lerp_x, lerp_y, 255);
                draw(pos.x, pos.y, 80);
            });

        // draw player
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
                                player_pos.x + player_vel.x * ctx.partial_ticks,
                                player_pos.y + player_vel.y * ctx.partial_ticks,
                            );
                            let (x2, y2) = (
                                grapple_pos.x + grapple_vel.x * ctx.partial_ticks,
                                grapple_pos.y + grapple_vel.y * ctx.partial_ticks,
                            );

                            target.line(
                                (x1 as f32, y1 as f32),
                                (x2 as f32, y2 as f32),
                                Color::rgba(191, 191, 191, 255),
                                DrawParameters {
                                    polygon_mode: PolygonMode::Line,
                                    line_width: Some(ctx.client.camera_scale as f32),
                                    blend: Blend::alpha_blending(),
                                    ..Default::default()
                                },
                            );
                        } else {
                            {
                                let (x1, y1) = (
                                    grapple_pos.x + grapple_vel.x * ctx.partial_ticks,
                                    grapple_pos.y + grapple_vel.y * ctx.partial_ticks,
                                );
                                let (x2, y2) = (pivots[0].x, pivots[0].y);
                                target.line(
                                    (x1 as f32, y1 as f32),
                                    (x2 as f32, y2 as f32),
                                    Color::rgba(191, 191, 191, 255),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(ctx.client.camera_scale as f32),
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
                                            line_width: Some(ctx.client.camera_scale as f32),
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
                                    player_pos.x + player_vel.x * ctx.partial_ticks,
                                    player_pos.y + player_vel.y * ctx.partial_ticks,
                                );
                                target.line(
                                    (x1 as f32, y1 as f32),
                                    (x2 as f32, y2 as f32),
                                    Color::rgba(191, 191, 191, 255),
                                    DrawParameters {
                                        polygon_mode: PolygonMode::Line,
                                        line_width: Some(ctx.client.camera_scale as f32),
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

    fn draw_chunk_overlays(
        &mut self,
        screen_zone: &Rect<i32>,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
        ctx: &RenderContext,
    ) {
        profiling::scope!("draw_chunk_overlays");
        let mut structure_lines = vec![];
        let mut state_rects = vec![];

        unsafe { world.chunk_handler.manager.raw_mut().iter_mut() }.for_each(|(_i, ch)| {
            let world_x = ch.chunk_x() * i32::from(CHUNK_SIZE);
            let world_y = ch.chunk_y() * i32::from(CHUNK_SIZE);
            let rc = Rect::new_wh(world_x, world_y, CHUNK_SIZE, CHUNK_SIZE);

            // queue structure set debug
            if let (true, Some(set)) = (ctx.settings.debug, ctx.settings.draw_structure_set.clone())
            {
                if let Some(v) = ctx.registries.structure_sets.get(&set) {
                    let (start_x, start_y) =
                        v.nearest_start_chunk((ch.chunk_x(), ch.chunk_y()), world.seed as _);
                    let should_gen_start = v.should_generate_at(
                        (start_x, start_y),
                        world.seed as _,
                        &ctx.registries,
                        true,
                    );
                    structure_lines.push((
                        (world_x as f32, world_y as f32),
                        (
                            (start_x * i32::from(CHUNK_SIZE)) as f32,
                            (start_y * i32::from(CHUNK_SIZE)) as f32,
                        ),
                        if start_x == ch.chunk_x() && start_y == ch.chunk_y() {
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
            target.transform.translate(world_x, world_y);

            if (ctx.settings.debug && !ctx.settings.cull_chunks) || rc.intersects(screen_zone) {
                ch.render(target, ctx.settings);

                // draw dirty rects
                if ctx.settings.debug && ctx.settings.draw_chunk_dirty_rects {
                    if let Some(dr) = ch.dirty_rect() {
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

                    if ch.graphics.pixels_updated_last_update {
                        let rect = Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE)
                            .into_f32()
                            .inflated(-2.0);
                        target.rectangle(
                            rect,
                            Color::rgba(255, 255, 64, 80),
                            DrawParameters {
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                        target.rectangle(
                            rect,
                            Color::rgba(255, 255, 64, 100),
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                    }

                    if ch.graphics.lighting_updated_last_update {
                        let rect = Rect::new_wh(0, 0, CHUNK_SIZE, CHUNK_SIZE)
                            .into_f32()
                            .inflated(-4.0);
                        target.rectangle(
                            rect,
                            Color::rgba(64, 255, 255, 32),
                            DrawParameters {
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                        target.rectangle(
                            rect,
                            Color::rgba(64, 255, 255, 64),
                            DrawParameters {
                                polygon_mode: PolygonMode::Line,
                                line_width: Some(1.0),
                                blend: Blend::alpha_blending(),
                                ..Default::default()
                            },
                        );
                    }

                    if let Some(dist) = ch.graphics.dist_to_nearest_dirty_light {
                        for i in 0..dist {
                            let rect = Rect::new_wh(20 + i * 12, 20, 10, 10).into_f32();
                            target.rectangle(
                                rect,
                                Color::rgba(255, 64, 255, 32),
                                DrawParameters {
                                    blend: Blend::alpha_blending(),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                }
            }

            // queue state overlay
            if ctx.settings.debug && ctx.settings.draw_chunk_state_overlay {
                let rect = Rect::new_wh(world_x, world_y, CHUNK_SIZE, CHUNK_SIZE);

                let alpha: u8 = (ctx.settings.draw_chunk_state_overlay_alpha * 255.0) as u8;
                let color = match ch.state() {
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
                state_rects.push((rect.into_f32(), color));
            }

            target.transform.pop();
        });

        // draw state overlay
        if ctx.settings.debug && ctx.settings.draw_chunk_state_overlay {
            target.rectangles_colored(
                &state_rects,
                DrawParameters {
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
            target.rectangles_colored(
                &state_rects,
                DrawParameters {
                    polygon_mode: PolygonMode::Line,
                    line_width: Some(1.0),
                    blend: Blend::alpha_blending(),
                    ..Default::default()
                },
            );
        }

        // draw structure set debug
        if ctx.settings.debug && ctx.settings.draw_structure_set.is_some() {
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
    }

    fn draw_rigidbodies(&mut self, world: &mut World<ClientChunk>, target: &mut RenderTarget) {
        profiling::scope!("draw_rigidbodies");
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

    fn draw_physics_debug(
        &mut self,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
        ctx: &RenderContext,
    ) {
        profiling::scope!("draw_physics_debug");
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

            // center of mass
            if ctx.settings.physics_dbg_draw_center_of_mass {
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
            }
            target.transform.pop();

            // colliders
            for c in b.colliders() {
                let col = world.physics.colliders.get(*c).unwrap();

                if ctx.settings.physics_dbg_draw_shape {
                    let shape = col.shape();
                    draw_shape(
                        shape,
                        rx,
                        ry,
                        b.rotation().angle(),
                        target,
                        Color::rgba(0x00, 0xff, 0x00, if b.is_sleeping() { 0x64 } else { 0xff }),
                    );
                }

                if ctx.settings.physics_dbg_draw_aabb {
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

    fn draw_chunk_grid(&mut self, camera_pos: &Position, target: &mut RenderTarget) {
        profiling::scope!("draw_chunk_grid");
        let rects = (-10..10)
            .into_iter()
            .flat_map(|x| {
                (-8..8).into_iter().map(move |y| {
                    let rc_x = x + (camera_pos.x / f64::from(CHUNK_SIZE)) as i32;
                    let rc_y = y + (camera_pos.y / f64::from(CHUNK_SIZE)) as i32;
                    let rc = Rect::new_wh(
                        rc_x * i32::from(CHUNK_SIZE),
                        rc_y * i32::from(CHUNK_SIZE),
                        CHUNK_SIZE,
                        CHUNK_SIZE,
                    );
                    rc.into_f32()
                })
            })
            .collect::<Vec<_>>();

        target.rectangles(
            &rects,
            Color::rgba(64, 64, 64, 127),
            DrawParameters {
                polygon_mode: PolygonMode::Line,
                line_width: Some(1.0),
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        );
    }

    fn draw_origin(&mut self, target: &mut RenderTarget) {
        profiling::scope!("draw_origin");
        const LEN: f32 = 16.0;
        target.rectangles_colored(
            &[
                // background X axis
                (
                    Rect::new_wh(-LEN - 2.0, -1.5, LEN * 2.0 + 4.0, 3.0),
                    Color::BLACK.with_a(0.5),
                ),
                // background Y axis
                (
                    Rect::new_wh(-1.5, -LEN - 2.0, 3.0, LEN * 2.0 + 4.0),
                    Color::BLACK.with_a(0.5),
                ),
                // X axis
                (
                    Rect::new_wh(-LEN - 1.0, -0.5, LEN * 2.0 + 2.0, 1.0),
                    Color::RED.with_a(0.5),
                ),
                // Y axis
                (
                    Rect::new_wh(-0.5, -LEN - 1.0, 1.0, LEN * 2.0 + 2.0),
                    Color::GREEN.with_a(0.5),
                ),
                // origin
                (Rect::new_wh(-1.0, -1.0, 2.0, 2.0), Color::WHITE),
            ],
            DrawParameters {
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        );
    }

    fn draw_load_zones(
        &mut self,
        loader: (f64, f64),
        camera: Option<(f64, f64)>,
        world: &mut World<ClientChunk>,
        target: &mut RenderTarget,
    ) {
        profiling::scope!("draw_load_zones");
        let active_zone = world.chunk_handler.get_active_zone(loader);
        let load_zone = world.chunk_handler.get_load_zone(loader);
        let unload_zone = world.chunk_handler.get_unload_zone(loader);

        let mut rects = vec![
            (unload_zone.into_f32(), Color::rgba(255, 0, 0, 127)),
            (load_zone.into_f32(), Color::rgba(255, 127, 0, 127)),
            (active_zone.into_f32(), Color::rgba(255, 255, 0, 127)),
        ];

        if let Some(camera) = camera {
            let screen_zone = world.chunk_handler.get_screen_zone(camera); // note we always use the camera for the screen zone
            rects.push((screen_zone.into_f32(), Color::rgba(0, 255, 0, 127)));
        }

        target.rectangles_colored(
            &rects,
            DrawParameters {
                polygon_mode: PolygonMode::Line,
                line_width: Some(1.0),
                blend: Blend::alpha_blending(),
                ..Default::default()
            },
        );
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

pub struct RenderContext<'a> {
    pub delta_time: f64,
    pub settings: &'a Settings,
    pub client: &'a mut Client,
    pub partial_ticks: f64,
    pub registries: Arc<Registries>,
    pub file_helper: &'a FileHelper,
}
