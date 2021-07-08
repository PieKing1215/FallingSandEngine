
mod game;
use game::Game;

use crate::game::Renderer;

#[profiling::function]
fn main() {
    println!("Hello, world!");

    #[cfg(feature = "profile-with-tracy")]
    {
        println!("Profiler Enabled");
    }

    // TODO: come up with a better way to handle this sdl's lifetime
    let sdl = Renderer::init_sdl().unwrap();
    let mut game: Game = Game::new();

    println!("Starting init...");
    {
        let init = game.init(&sdl);
        if let Err(s) = init {
            eprintln!("Error during init: {}", s);
            return;
        }
    }
    println!("Finished init.");

    println!("Starting main loop...");
    game.run(&sdl);
    println!("Goodbye!");

}
