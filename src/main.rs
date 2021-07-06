use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

fn main() {
    println!("Hello, world!");
    let r_init = sdl2::init();
    
    if r_init.is_err() {
        eprintln!("sdl2::init() FAILED {}", r_init.clone().err().unwrap());
        return;
    }

    let sdl_context = r_init.unwrap();

    let r_video = sdl_context.video();

    if r_video.is_err() {
        eprintln!("sdl_context.video() FAILED {}", r_video.unwrap_err());
        return;
    }

    let sdl_video = r_video.unwrap();

    let sdl_video = sdl_context.video().unwrap();
    let window = sdl_video.window("Window", 800, 600)
        .opengl() // this line DOES NOT enable opengl, but allows you to create/get an OpenGL context from your window.
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .build()
        .unwrap();

    canvas.set_draw_color(Color::RGBA(0, 0, 0, 0));
    canvas.clear();
    canvas.present();

    init();

    let mut prev_frame_time = std::time::Instant::now();

    let mut event_pump = sdl_context.event_pump().unwrap();
    'mainLoop: loop {

        // event handling
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'mainLoop
                },
                _ => {}
            }
        }

        // tick
        let now = std::time::Instant::now();
        if now.saturating_duration_since(prev_frame_time).as_nanos() > 1_000_000_000 / 30 { // 30 ticks per second
            prev_frame_time = now;
            tick();
        }

        // render
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();

        render(&mut canvas);

        canvas.present();

        // sleep
        ::std::thread::sleep(Duration::new(0, 1_000_000)); // 1ms
    }

    println!("Closing...");

}

fn init(){
    println!("Initializing...");
}

fn tick(){

}

fn render(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>){
    canvas.set_draw_color(Color::RGBA(255, 0, 0, 255));
    canvas.draw_rect(sdl2::rect::Rect::new(10, 10, 20, 20)).unwrap();
}
