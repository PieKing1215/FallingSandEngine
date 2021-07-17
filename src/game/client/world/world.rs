use liquidfun::box2d::common::{b2draw::b2ParticleColor, math::Vec2};
use sdl2::{libc, pixels::Color, rect::Rect};
use sdl_gpu::{GPURect, GPUSubsystem, GPUTarget, shaders::Shader};

use crate::game::{client::render::{Fonts, RenderCanvas, Renderable, Sdl2Context, Shaders, TransformStack}, common::{Settings, world::{CHUNK_SIZE, ChunkState, LIQUIDFUN_SCALE, World, gen::WorldGenerator}}};

impl World {
    pub fn render(&mut self, target: &mut GPUTarget, transform: &mut TransformStack, sdl: &Sdl2Context, fonts: &Fonts, settings: &Settings, shaders: &Shaders) {

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
                    ChunkState::NotGenerated => {
                        color = Color::RGBA(127, 127, 127, alpha);
                    },
                    ChunkState::Generating(stage) => {
                        color = Color::RGBA(64, (stage as f32 / self.chunk_handler.generator.max_gen_stage() as f32 * 255.0) as u8, 255, alpha);
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

        let mut liquid_target = self.liquid_image.get_target();
        liquid_target.clear();

        let particle_system = self.lqf_world.get_particle_system_list().unwrap();

        let particle_count = particle_system.get_particle_count();
        let particle_colors: &[b2ParticleColor] = particle_system.get_color_buffer();
        let particle_positions: &[Vec2] = particle_system.get_position_buffer();

        for i in 0..particle_count as usize {
            let pos = particle_positions[i];
            let color = particle_colors[i];
            let cam_x = self.camera.x.floor();
            let cam_y = self.camera.y.floor();
            GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_SET);
            let color = Color::RGBA(color.r, color.g, color.b, color.a);
            // let color = Color::RGBA(64, 90, 255, 191);
            liquid_target.pixel(pos.x * LIQUIDFUN_SCALE - cam_x as f32 + 1920.0/4.0 - 1.0, pos.y * LIQUIDFUN_SCALE - cam_y as f32 + 1080.0/4.0 - 1.0, color);
            // liquid_target.circle_filled(pos.x * 2.0 - self.camera.x as f32 + 1920.0/4.0, pos.y * 2.0 - self.camera.y as f32 + 1080.0/4.0, 2.0, Color::RGB(100, 100, 255));
        }

        GPUSubsystem::set_shape_blend_mode(sdl_gpu::sys::GPU_BlendPresetEnum::GPU_BLEND_NORMAL);
        shaders.liquid_shader.activate();
        self.liquid_image.blit_rect(None, target, Some(transform.transform_rect(screen_zone)));
        Shader::deactivate();

        // solids

        let transform_ptr: *const TransformStack = transform;
        let transform_ptr_raw = transform_ptr as usize;

        let canvas_ptr: *mut RenderCanvas = target;
        let canvas_ptr_raw = canvas_ptr as usize;

        let mut data = Some((canvas_ptr_raw, transform_ptr_raw));

        if settings.lqf_dbg_draw {
            transform.push();
            transform.scale(LIQUIDFUN_SCALE, LIQUIDFUN_SCALE);
            self.lqf_world.debug_draw(&mut data as *mut _ as *mut libc::c_void);
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