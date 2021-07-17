use std::{iter, ptr::slice_from_raw_parts};

use crate::game::common::Settings;
use liquidfun::box2d::{collision::shapes::polygon_shape::PolygonShape, common::{b2draw::{self, B2Draw_New, b2Color, b2ParticleColor, b2Transform, b2Vec2, int32}, math::Vec2}, dynamics::{body::{BodyDef, BodyType}, fixture::FixtureDef}, particle::{ParticleDef, TENSILE_PARTICLE, particle_system::ParticleSystemDef}};
use sdl2::pixels::Color;
use sdl_gpu::{GPUImage, GPUSubsystem, sys::GPU_FilterEnum, sys::GPU_FormatEnum};

use crate::game::client::render::TransformStack;
use crate::game::client::render::RenderCanvas;

use super::{ChunkHandler, gen::{TEST_GENERATOR, TestGenerator}};

pub const LIQUIDFUN_SCALE: f32 = 10.0;

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
        _radius: b2draw::float32,
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

        let axis_scale = 1.0;
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

        for i in 0..25000 {
            if i < 12500 {
                pd.color.set(255, 200, 64, 191);
            }else {
                pd.color.set(64, 200, 255, 191);
            }
            pd.position.set(-25.0 + (i as f32 / 100.0) * 0.17, -6.0 - ((i % 100) as f32) * 0.17);
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

