
mod game;

use std::str::FromStr;

use clap::App;
use clap::Arg;
use clap::crate_authors;
use clap::crate_name;
use clap::crate_version;
use game::Game;
use tui::Terminal;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;

use crate::game::client::Client;
use crate::game::client::render::Renderer;
use crate::game::client::render::Fonts;
use crate::game::client::world::ClientChunk;
use crate::game::client::world::ClientWorld;
use crate::game::common::world::entity::Entity;
use crate::game::server::world::ServerChunk;

fn is_type<T: FromStr>(val: String) -> Result<(), String>
where <T as std::str::FromStr>::Err : std::string::ToString {
    match val.parse::<T>() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[profiling::function]
fn main() -> Result<(), String> {
    let matches = App::new(crate_name!())
    .version(crate_version!())
    .author(crate_authors!())
    .arg(Arg::with_name("debug")
        .short("d")
        .long("debug")
        .help("Enable debugging features"))
    .arg(Arg::with_name("connect")  
        .short("c")
        .long("connect")
        .takes_value(true)
        .value_name("IP:PORT")
        .help("Connect to a server automatically"))
    .subcommand(App::new("server")
        .about("Run dedicated server")
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .takes_value(true)
            .value_name("PORT")
            .default_value("6673")
            .validator(is_type::<u16>)
            .help("The port to run the server on")))
    .get_matches();

    #[cfg(feature = "profile-with-tracy")]
    {
        println!("Profiler Enabled");
    }

    let server = matches.subcommand_matches("server").is_some();
    let client = !server;

    if server {

        crossterm::terminal::enable_raw_mode().unwrap();

        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);

        let mut terminal = Terminal::new(backend).unwrap();

        std::panic::set_hook(Box::new(|info| {
            let stdout = std::io::stdout();
            let mut backend = CrosstermBackend::new(stdout);
            backend.clear().unwrap();
            backend.set_cursor(0, 0).unwrap();

            let mut c: tui::buffer::Cell = tui::buffer::Cell::default();
            c.set_symbol(format!("{}\n", info).as_str());
            c.set_fg(tui::style::Color::LightRed);
            let t: (u16, u16, _) = (0, 0, &c);

            backend.draw(std::iter::once(t)).unwrap();
            backend.flush().unwrap();

            log::error!("{}", info);
        }));

        let res = std::panic::catch_unwind(move || {
            println!("Starting server...");
            let mut game: Game<ServerChunk> = Game::new();

            if let Some(w) = &mut game.world {
                w.add_entity(Entity {
                    x: 0.0,
                    y: 0.0,
                });
            };

            println!("Starting main loop...");
            match game.run(&matches, &mut terminal) {
                Ok(_) => {},
                Err(e) => panic!("Server encountered a fatal error: {}", e),
            }
        });

        if let Err(_) = res {
            println!("Server crashed, exiting...");
        }else{
            println!("Server shut down successfully.");
        }
    }else if client {
        println!("Starting client...");

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

        let mut game: Game<ClientChunk> = Game::new();
        
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
        game.run(&sdl, Some(&mut r), &matches);
        println!("Goodbye!");
    }

    Ok(())
}
