use std::{
    fs::File,
    sync::{mpsc, Arc},
};

use clap::Parser;
use interfaces::dbus_client::DBusClient;
use log::info;
use models::{args::Args, config::Config};
use services::{
    dbus_monitor::DBusMonitor, display::Display, player_manager::PlayerManager, runnable::Runnable,
};
use simplelog::{CombinedLogger, Config as LogConfig, WriteLogger};

mod effects;
mod helpers;
mod interfaces;
mod models;
mod services;
mod utils;

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
    let args = Arc::new(Args::parse());
    init_logger(args.debug)?;

    let config = match Config::new() {
        Ok(config) => Arc::new(config),
        Err(err) => {
            eprintln!("{err}");
            return Err(Box::new(err));
        }
    };

    let (event_tx, event_rx) = mpsc::channel();
    let (display_tx, display_rx) = mpsc::channel();

    let dbus_client = Arc::new(DBusClient::new());

    let services: Vec<Arc<dyn Runnable>> = vec![
        Arc::new(DBusMonitor::new(
            args.clone(),
            event_tx,
            dbus_client.clone(),
        )),
        Arc::new(PlayerManager::new(
            event_rx,
            display_tx,
            dbus_client,
        )),
        Arc::new(Display::new(
            args,
            config,
            display_rx,
        )),
    ];

    let mut handles = vec![];
    for service in services {
        handles.push(service.run());
    }

    for handle in handles {
        let _ = handle.join();
    }

    info!("all threads stopped, stopping...");

    Ok(())
}
