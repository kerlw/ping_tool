#[macro_use]
extern crate clap;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate trackable;

use clap::Arg;
use futures::{pin_mut, stream::StreamExt};
use netdiag::{Bind, Ping, Pinger};
use slog::Logger;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::SourceLocation;
use sloggers::Build;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

macro_rules! try_parse {
    ($expr:expr) => {
        track_any_err!($expr.parse())
    };
}

// macro_rules! allow_err {
//     ($e:expr) => {
//         if let Err(err) = $e {
//             println!(
//                 "{:?}, {}:{}:{}:{}",
//                 err,
//                 module_path!(),
//                 file!(),
//                 line!(),
//                 column!()
//             );
//         } else {
//             println!("send success.");
//         }
//     };
// }

async fn start_ping(target: IpAddr, logger: Logger) {
    match Pinger::new(&Bind::default()).await {
        Err(e) => error!(logger, "== Could not create Pinger == {:?}", e),
        Ok(pinger) => {
            info!(logger, "== Starting ping {} ==", &target);
            loop {
                let ping = Ping {
                    addr: target,
                    count: usize::MAX,
                    expiry: Duration::from_millis(300),
                };

                let stream = pinger.ping(&ping).enumerate();
                pin_mut!(stream);

                while let Some((_, item)) = stream.next().await {
                    if item.unwrap() == None {
                        error!(logger, "ping {} timeout.", &target);
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("target")
                .help("target to ping")
                .multiple(true)
                .min_values(1)
                .required(true),
        )
        .arg(
            Arg::with_name("LOG_LEVEL")
                .long("log-level")
                .takes_value(true)
                .default_value("info")
                .possible_values(&["debug", "info", "warning", "error"]),
        )
        .get_matches();

    let log_level = try_parse!(matches.value_of("LOG_LEVEL").unwrap())?;
    let logger = track!(TerminalLoggerBuilder::new()
        .source_location(SourceLocation::None)
        .destination(Destination::Stderr)
        .level(log_level)
        .build())?;
    let targets: Vec<&str> = matches.values_of("target").unwrap().collect();
    let handles = targets
        .into_iter()
        .map(|target| IpAddr::from_str(target).unwrap())
        .map(|target| tokio::spawn(start_ping(target, logger.clone())));

    futures::future::join_all(handles).await;

    Ok(())
}
