
mod game;

use game::Game;

use crate::game::client::Client;
use crate::game::client::render::Renderer;
use crate::game::client::render::Fonts;
use crate::game::client::world::ClientWorld;
use crate::game::common::world::entity::Entity;

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

    println!("Finished init.");

    let mut game: Game = Game::new();
    
    if let Some(w) = &mut game.world {
        let pl_id = w.add_entity(Entity {
            x: 0.0,
            y: 0.0,
        });
        game.client = Some(Client::new());
        game.client.as_mut().unwrap().world = Some(ClientWorld {
            local_entity_id: Some(pl_id),
        });
    };

    println!("Starting main loop...");
    game.run(&sdl, Some(&mut r));
    println!("Goodbye!");

    Ok(())
}
