
mod game;

use game::Game;

use crate::game::{Fonts, Renderer};

extern crate liquidfun;

#[profiling::function]
fn main() -> Result<(), String> {
    println!("Hello, world!");

    #[cfg(feature = "profile-with-tracy")]
    {
        println!("Profiler Enabled");
    }

    // TODO: come up with a better way to handle this sdl's lifetime
    let sdl = Renderer::init_sdl().unwrap();


    println!("Starting init...");
    
    let mut r = Renderer::create(&sdl)?;

    let pixel_operator2 = sdl.sdl_ttf.load_font("./assets/font/pixel_operator/PixelOperator.ttf", 16).unwrap();
    let f = Some(Fonts {
        pixel_operator: pixel_operator2,
    });
    r.fonts = f;

    let texture_creator = r.canvas.get_mut().texture_creator();
    
    println!("Finished init.");

    let mut game: Game = Game::new();
    println!("Starting main loop...");
    game.run(&sdl, Some(&mut r), &texture_creator);
    println!("Goodbye!");

    Ok(())
}
