use clap::Parser;
use log::info;

mod cli;
mod logger;
mod mpd;
mod waybar;

use cli::Args;
use mpd::MpdMonitor;
use waybar::WaybarDisplay;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let _ = logger::init_logger(args.debug);

    info!("Starting waybar-module-mpd ({}:{})", args.host, args.port);


    let display = WaybarDisplay::new();
    let monitor = MpdMonitor::new(&args.host, args.port);

    monitor.run(|state| {
        let output = match state {
            Some((status, song)) => display.format_output(&args, status, song),
            None => display.format_stopped(&args),
        };
        display.print_if_changed(&output);
    });

    Ok(())
}



