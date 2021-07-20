use std::time::{Duration, Instant};

use crate::game::Game;

use super::world::ServerChunk;

impl Game<ServerChunk> {
    #[profiling::function]
    pub fn run(&mut self) {
        let mut prev_tick_time = std::time::Instant::now();
        let mut prev_tick_lqf_time = std::time::Instant::now();

        let mut counter_last_frame = Instant::now();

        let mut do_tick_next = false;
        let mut do_tick_lqf_next = false;
        '_mainLoop: loop {
            
            let now = std::time::Instant::now();

            // tick

            let can_tick = self.settings.tick;

            if do_tick_next && can_tick {
                prev_tick_time = now;
                let st = Instant::now();
                self.tick();
                self.fps_counter.tick_times.rotate_left(1);
                self.fps_counter.tick_times[self.fps_counter.tick_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;
            }
            do_tick_next = can_tick && now.saturating_duration_since(prev_tick_time).as_nanos() > 1_000_000_000 / self.settings.tick_speed as u128; // intended is 30 ticks per second

            // tick liquidfun

            let can_tick = self.settings.tick_lqf;

            if do_tick_lqf_next && can_tick {
                prev_tick_lqf_time = now;
                if let Some(w) = &mut self.world {
                    let st = Instant::now();
                    w.tick_lqf(&self.settings);
                    self.fps_counter.tick_lqf_times.rotate_left(1);
                    self.fps_counter.tick_lqf_times[self.fps_counter.tick_lqf_times.len() - 1] = Instant::now().saturating_duration_since(st).as_nanos() as f32;
                    
                }
            }
            do_tick_lqf_next = can_tick && now.saturating_duration_since(prev_tick_lqf_time).as_nanos() > 1_000_000_000 / self.settings.tick_lqf_speed as u128; // intended is 60 ticks per second

            // render

            self.fps_counter.frames += 1;
            if now.saturating_duration_since(self.fps_counter.last_update).as_millis() >= 1000 {
                self.fps_counter.display_value = self.fps_counter.frames;
                self.fps_counter.frames = 0;
                self.fps_counter.last_update = now;
                
                let nums: Vec<f32> = self.fps_counter.frame_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspf: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                let nums: Vec<f32> = self.fps_counter.tick_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_mspt: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                let nums: Vec<f32> = self.fps_counter.tick_lqf_times.iter().filter(|n| **n != 0.0).map(|f| *f).collect();
                let avg_msplqft: f32 = nums.iter().map(|f| f / 1_000_000.0).sum::<f32>() / nums.len() as f32;

                println!("FPS: {}, mspf: {:.2}, mspt: {:.2}, msplqft: {:.2}", self.fps_counter.display_value, avg_mspf, avg_mspt, avg_msplqft);
            }

            let time_nano = Instant::now().saturating_duration_since(counter_last_frame).as_nanos();
            self.fps_counter.frame_times.rotate_left(1);
            self.fps_counter.frame_times[self.fps_counter.frame_times.len() - 1] = time_nano as f32;

            profiling::finish_frame!();
            // sleep
            if !do_tick_next {
                profiling::scope!("sleep");
                // let now = Instant::now();

                // TODO: this sleep is sleeping for like 15ms at a time on my system; figure out what the correct way to handle loop timing is
                ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms sleep so the computer doesn't explode
                
                // println!("slept {}ms", Instant::now().saturating_duration_since(now).as_millis());
            }
            counter_last_frame = Instant::now();
        }

        println!("Closing...");
    }

    #[profiling::function]
    fn tick(&mut self){
        self.tick_time += 1;

        if let Some(w) = &mut self.world {
            w.tick(self.tick_time, &self.settings);
        }
    }
}