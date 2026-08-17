use std::{thread, time::Duration};

use clap::Parser;
use log::{debug, info, warn};

mod cli;
mod logger;
mod mpd;
mod waybar;

use cli::Args;
use mpd::MpdClient;
use waybar::WaybarDisplay;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let _ = logger::init_logger(args.debug);


    info!("Starting waybar-module-music for MPD ({}:{})", args.host, args.port);

    let display = WaybarDisplay::new();

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


