use std::{iter, ptr::slice_from_raw_parts};

use liquidfun::box2d::common::{b2draw::{self, B2Draw_New, b2Color, b2ParticleColor, b2Transform, b2Vec2, int32}, math::Vec2};
use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem, GPUTarget, shaders::Shader, sys::{GPU_FilterEnum, GPU_FormatEnum, GPU_SetBlendMode}};
use specs::{Join, WriteStorage};

use crate::game::{client::{Client, render::{Fonts, RenderCanvas, Renderable, Sdl2Context, Shaders, TransformStack}}, common::{Settings, world::{CHUNK_SIZE, ChunkHandlerGeneric, ChunkState, LIQUIDFUN_SCALE, Position, World, gen::WorldGenerator, particle::Particle}}};

use super::{ClientChunk, ClientWorld};

pub struct WorldRenderer {
    pub lqf_debug_draw_callbacks: b2draw::b2DrawCallbacks,
    pub liquid_image: GPUImage,
    pub liquid_image2: GPUImage,
    lqf_dirty: bool,
}

impl WorldRenderer {

    pub fn new() -> Self {
        let lqf_debug_draw_callbacks = b2draw::b2DrawCallbacks {
            polygonCallback: Some(BoxDraw::draw_polygon),
            solidPolygonCallback: Some(BoxDraw::draw_solid_polygon),
            circleCallback: Some(BoxDraw::draw_circle),
            solidCircleCallback: Some(BoxDraw::draw_solid_circle),
            particlesCallback: Some(BoxDraw::draw_particles),
            segmentCallback: Some(BoxDraw::draw_segment),
            transformCallback: Some(BoxDraw::draw_transform),
        };

        let mut liquid_image = GPUSubsystem::create_image(1920/2, 1080/2, GPU_FormatEnum::GPU_FORMAT_RGBA);
        liquid_image.set_image_filter(GPU_FilterEnum::GPU_FILTER_NEAREST);

        let mut liquid_image2 = GPUSubsystem::create_image(1920/2, 1080/2, GPU_FormatEnum::GPU_FORMAT_RGBA);
        liquid_image2.set_image_filter(GPU_FilterEnum::GPU_FILTER_NEAREST);

        Self {
            lqf_debug_draw_callbacks,
            liquid_image,
            liquid_image2,
            lqf_dirty: false,
        }
    }

    pub fn init(&self, world: &mut World<ClientChunk>) {
        unsafe {
            let cast = &mut *(B2Draw_New(self.lqf_debug_draw_callbacks));
            world.lqf_world.set_debug_draw(cast);
        }
    }

    #[warn(clippy::too_many_arguments)]
    #[warn(clippy::too_many_lines)]
    #[profiling::function]
    pub fn render(&mut self, world: &mut World<ClientChunk>, target: &mut GPUTarget, transform: &mut TransformStack, delta_time: f64, sdl: &Sdl2Context, fonts: &Fonts, settings: &Settings, shaders: &Shaders, client: &mut Option<Client>) {

        if world.lqf_world.get_debug_draw().is_none() {
            self.init(world);
        }

        // draw world

        if let Some(cl) = client {
            if let Some(e_id) = cl.world.as_ref().and_then(|cw| cw.local_entity_id) {
                if let Some(ent) = world.get_entity(e_id) {
                    cl.camera.x += (ent.x - cl.camera.x) * (delta_time * 10.0).clamp(0.0, 1.0);
                    cl.camera.y += (ent.y - cl.camera.y) * (delta_time * 10.0).clamp(0.0, 1.0);
                }
            }
        }

        let loader_pos = match client {
            Some(Client{world: Some(ClientWorld{local_entity_id: Some(eid)}), .. }) => {
                world.get_entity_mut(*eid).map_or_else(|| (client.as_mut().unwrap().camera.x, client.as_mut().unwrap().camera.y), |e| (e.x, e.y))
            },
            _ => (client.as_mut().unwrap().camera.x, client.as_mut().unwrap().camera.y)
        };

        let camera = &mut client.as_mut().unwrap().camera;

        transform.push();
        transform.translate(f64::from(target.width()) / 2.0, f64::from(target.height()) / 2.0);
        transform.scale(camera.scale, camera.scale);
        transform.translate(-camera.x, -camera.y);


        let screen_zone = world.chunk_handler.get_screen_zone((camera.x, camera.y)); // note we always use the camera for the screen zone
        let active_zone = world.chunk_handler.get_active_zone(loader_pos);
        let load_zone = world.chunk_handler.get_load_zone(loader_pos);
        let unload_zone = world.chunk_handler.get_unload_zone(loader_pos);

        // let clip = canvas.clip_rect();
        // if game.settings.cull_chunks {
        //     canvas.set_clip_rect(transform.transform_rect(screen_zone));
        // }

        world.chunk_handler.loaded_chunks.iter().for_each(|(_i, ch)| {
            let rc = Rect::new(ch.chunk_x * i32::from(CHUNK_SIZE), ch.chunk_y * i32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));
            if (settings.debug && !settings.cull_chunks) || rc.has_intersection(screen_zone){
                transform.push();
                transform.translate(ch.chunk_x * i32::from(CHUNK_SIZE), ch.chunk_y * i32::from(CHUNK_SIZE));
                ch.render(target, transform, sdl, fonts, settings);

                if settings.debug && settings.draw_chunk_dirty_rects {
                    if let Some(dr) = ch.dirty_rect {
                        let rect = transform.transform_rect(dr);
                        target.rectangle_filled2(rect, Color::RGBA(255, 64, 64, 127));
                        target.rectangle2(rect, Color::RGBA(255, 64, 64, 127));
                    }
                    if ch.graphics.was_dirty {
                        let rect = transform.transform_rect(Rect::new(0, 0, u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE)));
                        target.rectangle_filled2(rect, Color::RGBA(255, 255, 64, 127));
                        target.rectangle2(rect, Color::RGBA(255, 255, 64, 127));
                    }
                }

                transform.pop();
            }

            if settings.debug && settings.draw_chunk_state_overlay {
                let rect = transform.transform_rect(rc);

                let alpha: u8 = (settings.draw_chunk_state_overlay_alpha * 255.0) as u8;
                let color;
                match ch.state {
                    ChunkState::NotGenerated => {
                        color = Color::RGBA(127, 127, 127, alpha);
                    },
                    ChunkState::Generating(stage) => {
                        color = Color::RGBA(64, (f32::from(stage) / f32::from(world.chunk_handler.generator.max_gen_stage()) * 255.0) as u8, 255, alpha);
                    },
                    ChunkState::Cached => {
                        color = Color::RGBA(255, 127, 64, alpha);
                    },
                    ChunkState::Active => {
                        color = Color::RGBA(64, 255, 64, alpha);
                    },
                }
                target.rectangle_filled2(rect, color);
                target.rectangle2(rect, color);
            
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

        });

        // draw liquids

        if self.lqf_dirty {
            self.lqf_dirty = false;

            let mut liquid_target = self.liquid_image.get_target();
            liquid_target.clear();

            if let Some(particle_system) = world.lqf_world.get_particle_system_list() {
                let particle_count = particle_system.get_particle_count();
                let particle_colors: &[b2ParticleColor] = particle_system.get_color_buffer();
                let particle_positions: &[Vec2] = particle_system.get_position_buffer();

                for i in 0..particle_count as usize {
                    let pos = particle_positions[i];
                    let color = particle_colors[i];
                    let cam_x = camera.x.floor();
                    let cam_y = camera.y.floor();
                    GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_SET);
                    let color = Color::RGBA(color.r, color.g, color.b, color.a);
                    // let color = Color::RGBA(64, 90, 255, 191);
                    liquid_target.pixel(pos.x * LIQUIDFUN_SCALE - cam_x as f32 + 1920.0/4.0 - 1.0, pos.y * LIQUIDFUN_SCALE - cam_y as f32 + 1080.0/4.0 - 1.0, color);
                    // liquid_target.circle_filled(pos.x * 2.0 - camera.x as f32 + 1920.0/4.0, pos.y * 2.0 - camera.y as f32 + 1080.0/4.0, 2.0, Color::RGB(100, 100, 255));
                }

                GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);

                let mut liquid_target2 = self.liquid_image2.get_target();
                liquid_target2.clear();

                // TODO: add this method to sdl-gpu-rust
                unsafe {
                    GPU_SetBlendMode(&mut self.liquid_image.raw, sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_SET);
                }
                
                shaders.liquid_shader.activate();
                self.liquid_image.blit_rect(None::<GPURect>, &mut liquid_target2, None);
                Shader::deactivate();

                // TODO: add this method to sdl-gpu-rust
                unsafe {
                    GPU_SetBlendMode(&mut self.liquid_image.raw, sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);
                }
            };
        }

        // TODO: transforming screen zone here is not the right way to do this, it causes some jumping when x or y switch between + and -
        self.liquid_image2.blit_rect(None, target, Some(transform.transform_rect(screen_zone)));

        // draw solids

        {
            profiling::scope!("rigidbodies");
            transform.push();
            transform.scale(LIQUIDFUN_SCALE, LIQUIDFUN_SCALE);
            for rb in &mut world.rigidbodies {
                if rb.image.is_none() {
                    rb.update_image();
                }

                if let Some(body) = &rb.body {
                    if let Some(img) = &rb.image {
                        let pos = body.get_position();

                        let (width, height) = (f32::from(rb.width) / LIQUIDFUN_SCALE, f32::from(rb.height) / LIQUIDFUN_SCALE);

                        let mut rect = GPURect::new(pos.x, pos.y, width, height);

                        let (x1, y1) = transform.transform((rect.x, rect.y));
                        let (x2, y2) = transform.transform((rect.x + rect.w, rect.y + rect.h));
                        
                        rect = GPURect::new2(x1 as f32, y1 as f32, x2 as f32, y2 as f32);

                        img.blit_rect_x(None, target, 
                            Some(rect), 
                            body.get_angle().to_degrees(), 0.0, 0.0, 0);
                    }
                }
            }
            transform.pop();
        }

        // lqf debug draw

        let transform_ptr: *const TransformStack = transform;
        let transform_ptr_raw = transform_ptr as usize;

        let canvas_ptr: *mut RenderCanvas = target;
        let canvas_ptr_raw = canvas_ptr as usize;

        let mut data = Some((canvas_ptr_raw, transform_ptr_raw));

        if settings.debug && settings.lqf_dbg_draw {
            profiling::scope!("lqf debug");
            transform.push();
            transform.scale(LIQUIDFUN_SCALE, LIQUIDFUN_SCALE);
            world.lqf_world.debug_draw((&mut data as *mut Option<(usize, usize)>).cast::<std::ffi::c_void>());
            transform.pop();
        }

        
        {
            profiling::scope!("particles");
            let (
                particle_storage,
                position_storage,
            ) = world.ecs.system_data::<(
                WriteStorage<Particle>,
                WriteStorage<Position>,
            )>();

            (&particle_storage, &position_storage).join().for_each(|(p, pos)| {
                let (x1, y1) = transform.transform((pos.x - 0.5, pos.y - 0.5));
                let (x2, y2) = transform.transform((pos.x + 0.5, pos.y + 0.5));
                target.rectangle_filled(x1 as f32, y1 as f32, x2 as f32, y2 as f32, p.material.color);
            });
        }
        
        {
            profiling::scope!("entities");
            world.entities.iter().for_each(|(_id, e)| {
                transform.push();
                transform.translate(e.x, e.y);

                let (x1, y1) = transform.transform((-6.0, -10.0));
                let (x2, y2) = transform.transform((6.0, 10.0));

                target.rectangle(x1 as f32, y1 as f32, x2 as f32, y2 as f32, Color::RGBA(255, 0, 0, 255));

                transform.pop();
            });
        }
        // canvas.set_clip_rect(clip);
        
        if settings.debug && settings.draw_chunk_grid {
            for x in -10..10 {
                for y in -10..10 {
                    let rc_x = x + (camera.x / f64::from(CHUNK_SIZE)) as i32;
                    let rc_y = y + (camera.y / f64::from(CHUNK_SIZE)) as i32;
                    let rc = Rect::new(rc_x * i32::from(CHUNK_SIZE), rc_y * i32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE), u32::from(CHUNK_SIZE));
                    target.rectangle2(transform.transform_rect(rc), Color::RGBA(64, 64, 64, 127))
                }
            }
        }

        if settings.debug && settings.draw_origin {
            let len: f32 = 16.0;
            let origin = transform.transform((0, 0));
            target.rectangle_filled2(GPURect::new(origin.0 as f32 - len - 2.0, origin.1 as f32 - 1.0, (len * 2.0 + 4.0) as f32, 3.0), Color::RGBA(0, 0, 0, 127));
            target.rectangle_filled2(GPURect::new(origin.0 as f32 - 1.0, origin.1 as f32 - len - 2.0, 3.0, (len * 2.0 + 4.0) as f32), Color::RGBA(0, 0, 0, 127));

            target.line(origin.0 as f32 - len, origin.1 as f32, origin.0 as f32 + len, origin.1 as f32, Color::RGBA(255, 0, 0, 255));
            target.line(origin.0 as f32, origin.1 as f32 - len, origin.0 as f32, origin.1 as f32 + len, Color::RGBA(0, 255, 0, 255));
        }

        if settings.debug && settings.draw_load_zones {
            target.rectangle2(transform.transform_rect(unload_zone), Color::RGBA(255, 0, 0, 127));
            target.rectangle2(transform.transform_rect(load_zone), Color::RGBA(255, 127, 0, 127));
            target.rectangle2(transform.transform_rect(active_zone), Color::RGBA(255, 255, 0, 127));
            target.rectangle2(transform.transform_rect(screen_zone), Color::RGBA(0, 255, 0, 127));
        }

        transform.pop();

        // draw overlay

    }

    pub fn mark_liquid_dirty(&mut self) {
        self.lqf_dirty = true;
    }
}

pub struct BoxDraw {
}

type B2DebugDrawContext = Option<(usize, usize)>;

impl BoxDraw {
    pub unsafe extern "C" fn draw_polygon(
        vertices: *const b2Vec2,
        vertex_count: int32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let verts: Vec<f32> = slice_from_raw_parts(vertices, vertex_count as usize).as_ref().unwrap().iter().flat_map(|v| {
            let (x, y) = transform.transform((v.x, v.y));
            iter::once(x as f32).chain(iter::once(y as f32))
        }).collect();

        let col = *color;
        canvas.polygon(verts, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    pub unsafe extern "C" fn draw_solid_polygon(
        vertices: *const b2Vec2,
        vertex_count: int32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let verts: Vec<f32> = slice_from_raw_parts(vertices, vertex_count as usize).as_ref().unwrap().iter().flat_map(|v| {
            let (x, y) = transform.transform((v.x, v.y));
            iter::once(x as f32).chain(iter::once(y as f32))
        }).collect();

        let col = *color;
        canvas.polygon_filled(verts.clone(), Color::RGBA((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8, 127));
        canvas.polygon(verts, Color::RGBA((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8, 255));
    }

    pub unsafe extern "C" fn draw_circle(
        center: *const b2Vec2,
        radius: b2draw::float32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let col = *color;

        let (x, y) = transform.transform(((*center).x, (*center).y));
        let (x_plus_rad, _y_plus_rad) = transform.transform(((*center).x + radius, (*center).x));
        canvas.circle(x as f32, y as f32, (x_plus_rad - x) as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    pub unsafe extern "C" fn draw_solid_circle(
        center: *const b2Vec2,
        radius: b2draw::float32,
        _axis: *const b2Vec2,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let col = *color;

        let (x, y) = transform.transform(((*center).x, (*center).y));
        let (x_plus_rad, _y_plus_rad) = transform.transform(((*center).x + radius, (*center).x));
        canvas.circle_filled(x as f32, y as f32, (x_plus_rad - x) as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    pub unsafe extern "C" fn draw_particles(
        centers: *const b2Vec2,
        _radius: b2draw::float32,
        _colors: *const b2ParticleColor,
        count: int32,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let centers_vec: Vec<(f32, f32)> = slice_from_raw_parts(centers, count as usize).as_ref().unwrap().iter().map(|v| {
            (v.x, v.y)
        }).collect();

        // if colors.is_null() {
            for (x, y) in centers_vec {
                let col = Color::RGB(255, 100, 100);
                // let p1 = (x - radius, y - radius);
                // let p2 = (x + radius, y + radius);
                // let p1_i = transform.transform(p1);
                // let p2_i = transform.transform(p2);

                let center = transform.transform((x, y));
                // canvas.rectangle2(Rect::new(p1_i.0 as i32, p1_i.1 as i32, (p2_i.0 - p1_i.0) as u32, (p2_i.1 - p1_i.1) as u32), col);
                canvas.pixel(center.0 as f32, center.1 as f32, col);
                // canvas.filled_circle(x, y, x_plus_rad - x, col).unwrap();
            }
        // }else {
        //     let colors_vec: Vec<Color> = slice_from_raw_parts(colors, count as usize).as_ref().unwrap().iter().map(|col| {
        //         Color::RGBA(col.r, col.g, col.b, 255)
        //     }).collect();

        //     for i in 0..count as usize {
        //         let (x, y, x_plus_rad) = centers_vec[i];
        //         let col = colors_vec[i];
        //         ctx.0.filled_circle(x, y, x_plus_rad - x, col).unwrap();
        //     }
        // }
    }

    pub unsafe extern "C" fn draw_segment(
        p1: *const b2Vec2,
        p2: *const b2Vec2,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let col = *color;
        let pt1 = *p1;
        let pt2 = *p2;

        let (p1_x, p1_y) = transform.transform((pt1.x, pt1.y));
        let (p2_x, p2_y) = transform.transform((pt2.x, pt2.y));

        canvas.line(p1_x as f32, p1_y as f32, p2_x as f32, p2_y as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    pub unsafe extern "C" fn draw_transform(xf: *const b2Transform, user_data: *mut ::std::os::raw::c_void) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *user_data.cast::<B2DebugDrawContext>()).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let axis_scale = 1.0;
        let p1 = (*xf).p;

        {
            let p2 = b2Vec2 { x: (*xf).q.c * axis_scale + p1.x, y: (*xf).q.s * axis_scale + p1.y };
            let (p1_x, p1_y) = transform.transform((p1.x, p1.y));
            let (p2_x, p2_y) = transform.transform((p2.x, p2.y));
            canvas.line(p1_x as f32, p1_y as f32, p2_x as f32, p2_y as f32, Color::RGB(0xff, 0, 0));
        }

        {
            let p2 = b2Vec2 { x: -(*xf).q.s * axis_scale + p1.x, y: (*xf).q.c * axis_scale + p1.y };
            let (p1_x, p1_y) = transform.transform_int((p1.x, p1.y));
            let (p2_x, p2_y) = transform.transform_int((p2.x, p2.y));
            canvas.line(p1_x as f32, p1_y as f32, p2_x as f32, p2_y as f32, Color::RGB(0, 0xff, 0));
        }
    }
}