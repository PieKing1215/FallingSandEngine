use std::{fs::File, thread};

use backtrace::Backtrace;
use fs_client::{render::Renderer, world::ClientWorld, ClientGame};
use fs_common::game::{
    common::{
        cli::{CLArgs, CLSubcommand},
        world::{entity::Player, Camera, Target},
        FileHelper,
    },
    BuildData,
};
use fs_server::ServerGame;
use log::{error, info, LevelFilter};

// use salva2d::{integrations::rapier::ColliderSampling, object::Boundary};
use simplelog::{CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

pub fn main() -> Result<(), String> {
    #[cfg(feature = "profile")]
    profiling::tracy_client::Client::start();

    profiling::scope!("main");

    let build_data = BuildData {
        datetime: option_env!("BUILD_DATETIME"),
        git_hash: option_env!("GIT_HASH"),
    };

    let cl_args = CLArgs::parse_args();

    let file_helper = FileHelper::new(cl_args.game_dir.clone(), cl_args.assets_dir.clone());

    if !file_helper.game_path("").exists() {
        info!("game dir missing, creating it...");
        std::fs::create_dir_all(file_helper.game_path("")).expect("Failed to create game dir:");
    }

    if !file_helper.asset_path("").exists() {
        info!("asset dir missing, creating it...");
        std::fs::create_dir_all(file_helper.asset_path("")).expect("Failed to create asset dir:");
    }

    let server = matches!(cl_args.subcommand, Some(CLSubcommand::Server { .. }));
    let client = !server;

    let cpus = num_cpus::get();
    std::env::set_var(
        "RAYON_NUM_THREADS",
        format!("{}", (cpus - 4).max(2).min(cpus)),
    );

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

            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");
            let bt = Backtrace::new();

            let mut c: tui::buffer::Cell = tui::buffer::Cell::default();
            c.set_symbol(
                format!(
                    "thread '{name}' {info}\nSee latest.log for more details. Backtrace:\n{bt:?}\n"
                )
                .as_str(),
            );
            c.set_fg(tui::style::Color::LightRed);
            let t: (u16, u16, _) = (0, 0, &c);

            backend.draw(std::iter::once(t)).unwrap();
            backend.flush().unwrap();

            error!("thread '{}' {}", name, info);
            error!(
                "See server_latest.log for more details. Backtrace:\n{:?}",
                bt
            );
        }));

        let res = std::panic::catch_unwind(move || {
            println!("Starting server...");
            let mut game: ServerGame = ServerGame::new(file_helper, build_data);

            if let Some(w) = &mut game.0.world {
                Player::create_and_add(w);
            }

            println!("Starting main loop...");
            match game.run(&cl_args, &mut terminal) {
                Ok(_) => {},
                Err(e) => panic!("Server encountered a fatal error: {e}"),
            }
        });

        if res.is_err() {
            println!("Server crashed, exiting...");
        } else {
            println!("Server shut down successfully.");
        }
    } else if client {
        let debug = cl_args.debug;

        {
            profiling::scope!("Init logging");
            if !file_helper.game_path("logs/").exists() {
                info!("logs dir missing, creating it...");
                std::fs::create_dir_all(file_helper.game_path("logs/"))
                    .expect("Failed to create logs dir:");
            }

            CombinedLogger::init(vec![
                TermLogger::new(
                    if debug {
                        LevelFilter::Trace
                    } else {
                        LevelFilter::Info
                    },
                    ConfigBuilder::new()
                        .set_location_level(if debug {
                            LevelFilter::Error
                        } else {
                            LevelFilter::Off
                        })
                        .set_level_padding(simplelog::LevelPadding::Right)
                        .set_target_level(LevelFilter::Off)
                        .set_time_offset_to_local()
                        .unwrap()
                        .build(),
                    TerminalMode::Mixed,
                    simplelog::ColorChoice::Auto,
                ),
                WriteLogger::new(
                    LevelFilter::Trace,
                    ConfigBuilder::new()
                        .set_location_level(LevelFilter::Error)
                        .set_level_padding(simplelog::LevelPadding::Right)
                        .set_target_level(LevelFilter::Off)
                        .set_time_offset_to_local()
                        .unwrap()
                        .build(),
                    File::create(file_helper.game_path("logs/client_latest.log")).unwrap(),
                ),
            ])
            .unwrap();
        }

        std::panic::set_hook(Box::new(|info| {
            let thread = thread::current();
            let name = thread.name().unwrap_or("<unnamed>");

            error!("thread '{}' {}", name, info);
            error!(
                "See client_latest.log for more details. Backtrace:\n{:?}",
                Backtrace::new()
            );
        }));

        info!("Starting client...");

        info!("Starting init...");

        let event_loop = {
            profiling::scope!("EventLoop::new");
            glutin::event_loop::EventLoop::new()
        };
        let r = Renderer::create(&event_loop, &file_helper).expect("Renderer::create failed"); // want to panic

        info!("Finished init.");

        let mut game: ClientGame = ClientGame::new(file_helper, build_data);

        if let Some(w) = &mut game.data.world {
            let player = Player::create_and_add(w);

            Camera::create_and_add(w, Target::Entity(player));

            game.client.world = Some(ClientWorld { local_entity: Some(player) });
        };

        info!("Starting main loop...");
        game.run(r, cl_args, event_loop);
        info!("Goodbye!");
    }

    Ok(())
}
