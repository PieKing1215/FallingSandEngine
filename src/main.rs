
mod game;
use game::Game;

fn main() {
    println!("Hello, world!");

    let mut game: Game = Game::new();

    println!("Starting init...");
    let init = game.init();
    if let Err(s) = init {
        eprintln!("Error during init: {}", s);
        return;
    }
    println!("Finished init.");

    println!("Starting main loop...");
    game.run();
    println!("Goodbye!");

}
