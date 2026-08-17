use std::{fs::File, thread, time::Duration};

use clap::Parser;
use log::{debug, info, warn};
use simplelog::{CombinedLogger, Config as LogConfig, WriteLogger};

mod helpers;
mod models;
mod services;
mod utils;

use models::args::Args;
use services::display::Display;
use utils::mpd::MpdClient;

fn init_logger(debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = helpers::dir::get_and_create_dir(dirs::cache_dir)?;
    let log_path = cache_dir.join("app.log");

    CombinedLogger::init(vec![WriteLogger::new(
        if debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        },
        LogConfig::default(),
        File::create(log_path)?,
    )])?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let _ = init_logger(args.debug);

    info!("Starting waybar-module-music for MPD ({}:{})", args.host, args.port);

    let display = Display::new();

    // Print initial stopped state until connected
    display.print_if_changed(&display.format_stopped(&args));

    loop {
        debug!("Connecting to MPD at {}:{}", args.host, args.port);
        match MpdClient::connect(&args.host, args.port, Duration::from_secs(2)) {
            Ok(mut client) => {
                info!("Connected to MPD successfully");
                loop {
                    match client.query_status_and_song() {
                        Ok((status, song)) => {
                            debug!("MPD state: {:?}, song: {:?}", status.state, song.title);
                            let output = display.format_output(&args, &status, &song);
                            display.print_if_changed(&output);
                        }
                        Err(err) => {
                            warn!("Failed to query MPD status: {err}");
                            break;
                        }
                    }

                    // Block on MPD idle events (0% CPU, 0ms latency on changes)
                    if let Err(err) = client.wait_for_idle() {
                        warn!("MPD idle connection closed: {err}");
                        break;
                    }
                }
            }
            Err(err) => {
                debug!("Unable to connect to MPD: {err}");
            }
        }

        // Connection dropped or failed: output stopped status and wait before reconnect
        display.print_if_changed(&display.format_stopped(&args));
        thread::sleep(Duration::from_secs(2));
    }
}

