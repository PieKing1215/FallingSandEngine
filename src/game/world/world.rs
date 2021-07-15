use std::{ffi::c_void, iter, ptr::slice_from_raw_parts};

use crate::game::{Settings, world::gen::WorldGenerator};
use liquidfun::box2d::{collision::shapes::polygon_shape::PolygonShape, common::{b2draw::{self, B2Draw_New, b2Color, b2ParticleColor, b2Transform, b2Vec2, int32}, math::Vec2}, dynamics::{body::{BodyDef, BodyType}, fixture::FixtureDef}, particle::{ELASTIC_PARTICLE, ParticleDef, TENSILE_PARTICLE, particle_system::ParticleSystemDef}};
use sdl2::{pixels::Color, rect::Rect};
use sdl_gpu::{GPUImage, GPURect, GPUSubsystem, GPUTarget, sys::GPU_FormatEnum, sys::GPU_FilterEnum};

use crate::game::{Fonts, Game, RenderCanvas, Renderable, Sdl2Context, TransformStack};

use super::{CHUNK_SIZE, ChunkHandler, gen::{TEST_GENERATOR, TestGenerator}};

pub struct World {
    pub camera: Camera,
    pub chunk_handler: ChunkHandler<TestGenerator>,
    pub lqf_world: liquidfun::box2d::dynamics::world::World,
    pub lqf_debug_draw_callbacks: b2draw::b2DrawCallbacks,
    pub liquid_image: GPUImage,
}

pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

struct BoxDraw {
}

type B2DebugDrawContext = Option<(usize, usize)>;

impl BoxDraw {
    unsafe extern "C" fn draw_polygon(
        vertices: *const b2Vec2,
        vertex_count: int32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let verts: Vec<f32> = slice_from_raw_parts(vertices, vertex_count as usize).as_ref().unwrap().iter().flat_map(|v| {
            let (x, y) = transform.transform((v.x, v.y));
            iter::once(x as f32).chain(iter::once(y as f32))
        }).collect();

        let col = *color;
        canvas.polygon(verts, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    unsafe extern "C" fn draw_solid_polygon(
        vertices: *const b2Vec2,
        vertex_count: int32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let verts: Vec<f32> = slice_from_raw_parts(vertices, vertex_count as usize).as_ref().unwrap().iter().flat_map(|v| {
            let (x, y) = transform.transform((v.x, v.y));
            iter::once(x as f32).chain(iter::once(y as f32))
        }).collect();

        let col = *color;
        canvas.polygon_filled(verts, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    unsafe extern "C" fn draw_circle(
        center: *const b2Vec2,
        radius: b2draw::float32,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let col = *color;

        let (x, y) = transform.transform(((*center).x, (*center).y));
        let (x_plus_rad, _y_plus_rad) = transform.transform(((*center).x + radius, (*center).x));
        canvas.circle(x as f32, y as f32, (x_plus_rad - x) as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    unsafe extern "C" fn draw_solid_circle(
        center: *const b2Vec2,
        radius: b2draw::float32,
        _axis: *const b2Vec2,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let col = *color;

        let (x, y) = transform.transform(((*center).x, (*center).y));
        let (x_plus_rad, _y_plus_rad) = transform.transform(((*center).x + radius, (*center).x));
        canvas.circle_filled(x as f32, y as f32, (x_plus_rad - x) as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    unsafe extern "C" fn draw_particles(
        centers: *const b2Vec2,
        radius: b2draw::float32,
        _colors: *const b2ParticleColor,
        count: int32,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let centers_vec: Vec<(f32, f32)> = slice_from_raw_parts(centers, count as usize).as_ref().unwrap().iter().map(|v| {
            (v.x, v.y)
        }).collect();

        // if colors.is_null() {
            for i in 0..count as usize {
                let (x, y) = centers_vec[i];
                let col = Color::RGB(100, 100, 255);
                let p1 = (x - radius, y - radius);
                let p2 = (x + radius, y + radius);
                let p1_i = transform.transform(p1);
                let p2_i = transform.transform(p2);
                canvas.rectangle_filled2(Rect::new(p1_i.0 as i32, p1_i.1 as i32, (p2_i.0 - p1_i.0) as u32, (p2_i.1 - p1_i.1) as u32), col);
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

    unsafe extern "C" fn draw_segment(
        p1: *const b2Vec2,
        p2: *const b2Vec2,
        color: *const b2Color,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);
        
        let col = *color;
        let pt1 = *p1;
        let pt2 = *p2;

        let (p1x, p1y) = transform.transform((pt1.x, pt1.y));
        let (p2x, p2y) = transform.transform((pt2.x, pt2.y));

        canvas.line(p1x as f32, p1y as f32, p2x as f32, p2y as f32, Color::RGB((col.r * 255.0) as u8, (col.g * 255.0) as u8, (col.b * 255.0) as u8));
    }

    unsafe extern "C" fn draw_transform(xf: *const b2Transform, user_data: *mut ::std::os::raw::c_void) {
        let (canvas_ptr_raw, transform_ptr_raw) = (&mut *(user_data as *mut B2DebugDrawContext)).unwrap();
        let canvas = &mut *(canvas_ptr_raw as *mut RenderCanvas);
        let transform = &*(transform_ptr_raw as *const TransformStack);

        let axis_scale = 8.0;
        let p1 = (*xf).p;

        let p2 = b2Vec2 { x: (*xf).q.c * axis_scale + p1.x, y: (*xf).q.s * axis_scale + p1.y };
        let (p1_x, p1_y) = transform.transform((p1.x, p1.y));
        let (p2_x, p2_y) = transform.transform((p2.x, p2.y));
        canvas.line(p1_x as f32, p1_y as f32, p2_x as f32, p2_y as f32, Color::RGB(0xff, 0, 0));

        let p2 = b2Vec2 { x: -(*xf).q.s * axis_scale + p1.x, y: (*xf).q.c * axis_scale + p1.y };
        let (p1_x, p1_y) = transform.transform_int((p1.x, p1.y));
        let (p2_x, p2_y) = transform.transform_int((p2.x, p2.y));
        canvas.line(p1_x as f32, p1_y as f32, p2_x as f32, p2_y as f32, Color::RGB(0, 0xff, 0));
    }
}

impl<'w> World {
    #[profiling::function]
    pub fn create() -> Self {
        let gravity = liquidfun::box2d::common::math::Vec2::new(0.0, 10.0);
        let mut lqf_world = liquidfun::box2d::dynamics::world::World::new(&gravity);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(0.0, 10.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(100.0, 2.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(100.0, -20.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(2.0, 50.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(-100.0, -20.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(2.0, 50.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut ground_body_def = BodyDef::default();
	    ground_body_def.position.set(70.0, -55.0);
        let ground_body = lqf_world.create_body(&ground_body_def);
        let mut ground_box = PolygonShape::new();
        ground_box.set_as_box(2.0, 50.0);
        ground_body.create_fixture_from_shape(&ground_box, 0.0);

        let mut body_def = BodyDef::default();
        body_def.body_type = BodyType::DynamicBody;
        body_def.position.set(0.0, -160.0);
        body_def.angular_velocity = 3.0;
        body_def.linear_velocity = Vec2::new(0.0, -10.0);
        let body = lqf_world.create_body(&body_def);
        let mut dynamic_box = PolygonShape::new();
        dynamic_box.set_as_box(10.0, 10.0);
        let mut fixture_def = FixtureDef::new(&dynamic_box);
        fixture_def.density = 0.5;
        fixture_def.friction = 0.3;
        body.create_fixture(&fixture_def);

        let mut particle_system_def = ParticleSystemDef::default();
        particle_system_def.radius = 0.5;
        particle_system_def.color_mixing_strength = 0.0;
	    let particle_system = lqf_world.create_particle_system(&particle_system_def);
        let mut pd = ParticleDef::default();
        pd.flags.insert(ELASTIC_PARTICLE | TENSILE_PARTICLE);
        pd.color.set(255, 255, 255, 255);

        for i in 0..10000 {
            pd.position.set(-90.0 + (i as f32 / 50.0) * 0.8, -1.0 - ((i % 50) as f32) * 0.8);
            particle_system.create_particle(&pd);
        }

        let callbacks = b2draw::b2DrawCallbacks {
            polygonCallback: Some(BoxDraw::draw_polygon),
            solidPolygonCallback: Some(BoxDraw::draw_solid_polygon),
            circleCallback: Some(BoxDraw::draw_circle),
            solidCircleCallback: Some(BoxDraw::draw_solid_circle),
            particlesCallback: Some(BoxDraw::draw_particles),
            segmentCallback: Some(BoxDraw::draw_segment),
            transformCallback: Some(BoxDraw::draw_transform),
        };
        

        unsafe {
            let cast = &mut *(B2Draw_New(callbacks));
            lqf_world.set_debug_draw(cast);
        }

        let mut liquid_image = GPUSubsystem::create_image(1920/2, 1080/2, GPU_FormatEnum::GPU_FORMAT_RGBA);
        liquid_image.set_image_filter(GPU_FilterEnum::GPU_FILTER_NEAREST);

        World {
            camera: Camera {
                x: 0.0,
                y: 0.0,
                scale: 2.0,
            },
            chunk_handler: ChunkHandler::new(TEST_GENERATOR),
            lqf_world,
            lqf_debug_draw_callbacks: callbacks,
            liquid_image
        }
    }

    #[profiling::function]
    pub fn tick(&mut self, tick_time: u32, settings: &Settings){
        self.chunk_handler.tick(tick_time, &self.camera, settings);
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
                    // handled separately
                    // if settings.lqf_dbg_draw_particle {
                    //     cast.AppendFlags(b2draw::b2Draw_e_particleBit as u32);
                    // }
                }
            }
        }

        let time_step = settings.tick_lqf_timestep;
        let velocity_iterations = 6;
        let position_iterations = 2;
        self.lqf_world.step(time_step, velocity_iterations, position_iterations);
    }

    pub fn render(&mut self, target: &mut GPUTarget, transform: &mut TransformStack, sdl: &Sdl2Context, fonts: &Fonts, settings: &Settings) {

        // draw world

        transform.push();
        transform.translate(target.width() as f64 / 2.0, target.height() as f64 / 2.0);
        transform.scale(self.camera.scale, self.camera.scale);
        transform.translate(-self.camera.x, -self.camera.y);

        let screen_zone = self.chunk_handler.get_screen_zone(&self.camera);
        let active_zone = self.chunk_handler.get_active_zone(&self.camera);
        let load_zone = self.chunk_handler.get_load_zone(&self.camera);
        let unload_zone = self.chunk_handler.get_unload_zone(&self.camera);

        // let clip = canvas.clip_rect();
        // if game.settings.cull_chunks {
        //     canvas.set_clip_rect(transform.transform_rect(screen_zone));
        // }

        self.chunk_handler.loaded_chunks.iter().for_each(|(_i, ch)| {
            let rc = Rect::new(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
            if !settings.cull_chunks || rc.has_intersection(screen_zone){
                transform.push();
                transform.translate(ch.chunk_x * CHUNK_SIZE as i32, ch.chunk_y * CHUNK_SIZE as i32);
                ch.render(target, transform, sdl, fonts);

                if settings.draw_chunk_dirty_rects {
                    if let Some(dr) = ch.dirty_rect {
                        let rect = transform.transform_rect(dr);
                        target.rectangle_filled2(rect, Color::RGBA(255, 64, 64, 127));
                        target.rectangle2(rect, Color::RGBA(255, 64, 64, 127));
                    }
                    if ch.graphics.was_dirty {
                        let rect = transform.transform_rect(Rect::new(0, 0, CHUNK_SIZE as u32, CHUNK_SIZE as u32));
                        target.rectangle_filled2(rect, Color::RGBA(255, 255, 64, 127));
                        target.rectangle2(rect, Color::RGBA(255, 255, 64, 127));
                    }
                }

                transform.pop();
            }

            if settings.draw_chunk_state_overlay {
                let rect = transform.transform_rect(rc);

                let alpha: u8 = (settings.draw_chunk_state_overlay_alpha * 255.0) as u8;
                let color;
                match ch.state {
                    super::ChunkState::NotGenerated => {
                        color = Color::RGBA(127, 127, 127, alpha);
                    },
                    super::ChunkState::Generating(stage) => {
                        color = Color::RGBA(64, (stage as f32 / self.chunk_handler.generator.max_gen_stage() as f32 * 255.0) as u8, 255, alpha);
                    },
                    super::ChunkState::Cached => {
                        color = Color::RGBA(255, 127, 64, alpha);
                    },
                    super::ChunkState::Active => {
                        color = Color::RGBA(64, 255, 64, alpha);
                    },
                }
                target.rectangle_filled2(rect, color);
                target.rectangle2(rect, color);
            
                // let ind = self.chunk_handler.chunk_index(ch.chunk_x, ch.chunk_y);
                // let ind = self.chunk_handler.chunk_update_order(ch.chunk_x, ch.chunk_y);
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

        // TODO: this doesn't need to render every frame
        if settings.lqf_dbg_draw {
            // liquids

            if settings.lqf_dbg_draw_particle {
                let mut liquid_target = self.liquid_image.get_target();
                liquid_target.clear();

                let particle_system = self.lqf_world.get_particle_system_list().unwrap();

                let particle_count = particle_system.get_particle_count();
                let particle_colors: &[b2ParticleColor] = particle_system.get_color_buffer();
                let particle_positions: &[Vec2] = particle_system.get_position_buffer();

                for i in 0..particle_count as usize {
                    let pos = particle_positions[i];
                    let color = particle_colors[i];
                    liquid_target.circle_filled(pos.x * 2.0 - self.camera.x as f32 + 1920.0/4.0, pos.y * 2.0 - self.camera.y as f32 + 1080.0/4.0, 2.0, Color::RGB(100, 100, 255));
                }
                self.liquid_image.blit_rect(None, target, Some(transform.transform_rect(screen_zone)));
            }

            // solids

            transform.push();
            transform.scale(2.0, 2.0);

            let transform_ptr: *const TransformStack = transform;
            let transform_ptr_raw = transform_ptr as usize;

            let canvas_ptr: *mut RenderCanvas = target;
            let canvas_ptr_raw = canvas_ptr as usize;

            let mut data = Some((canvas_ptr_raw, transform_ptr_raw));

            self.lqf_world.debug_draw(&mut data as *mut _ as *mut c_void);

            transform.pop();


        }

        // canvas.set_clip_rect(clip);
        
        if settings.draw_chunk_grid {
            for x in -10..10 {
                for y in -10..10 {
                    let rcx = x + (self.camera.x / CHUNK_SIZE as f64) as i32;
                    let rcy = y + (self.camera.y / CHUNK_SIZE as f64) as i32;
                    let rc = Rect::new(rcx * CHUNK_SIZE as i32, rcy * CHUNK_SIZE as i32, CHUNK_SIZE as u32, CHUNK_SIZE as u32);
                    target.rectangle2(transform.transform_rect(rc), Color::RGBA(64, 64, 64, 127))
                }
            }
        }

        if settings.draw_origin {
            let len: f32 = 16.0;
            let origin = transform.transform((0, 0));
            target.rectangle_filled2(GPURect::new(origin.0 as f32 - len - 2.0, origin.1 as f32 - 1.0, (len * 2.0 + 4.0) as f32, 3.0), Color::RGBA(0, 0, 0, 127));
            target.rectangle_filled2(GPURect::new(origin.0 as f32 - 1.0, origin.1 as f32 - len - 2.0, 3.0, (len * 2.0 + 4.0) as f32), Color::RGBA(0, 0, 0, 127));

            target.line(origin.0 as f32 - len, origin.1 as f32, origin.0 as f32 + len, origin.1 as f32, Color::RGBA(255, 0, 0, 255));
            target.line(origin.0 as f32, origin.1 as f32 - len, origin.0 as f32, origin.1 as f32 + len, Color::RGBA(0, 255, 0, 255));
        }

        if settings.draw_load_zones {
            target.rectangle2(transform.transform_rect(unload_zone), Color::RGBA(255, 0, 0, 127));
            target.rectangle2(transform.transform_rect(load_zone), Color::RGBA(255, 127, 0, 127));
            target.rectangle2(transform.transform_rect(active_zone), Color::RGBA(255, 255, 0, 127));
            target.rectangle2(transform.transform_rect(screen_zone), Color::RGBA(0, 255, 0, 127));
        }

        transform.pop();

        // draw overlay


    }
}

