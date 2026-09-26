mod blocklist;
mod config;
mod locallogger;
mod semaphore;
mod worker;

use crate::blocklist::BlockList;
use crate::config::Config;
use crate::locallogger::LocalLogger;
use crate::semaphore::Semaphore;
use crate::worker::{parse_url_to_parts, run_download};

use log::{error, info, warn};
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc::sync_channel};
use std::{cmp, env, thread};

/// Run the actual blocklist generation. Effectively, this will take the config,
/// download all blocklists and merge them into one. At the end, the generated
/// blocklist is returned.
fn run_blocklist_generation(config: &Config) -> BlockList {
    let mut blocklist = blocklist::BlockList::new();
    let (node_sender, node_receiver) = sync_channel::<BlockList>(32);
    let max_parallel_download: usize = match config.max_parallel_downloads {
        Some(max_parallel_downloads) => {
            if max_parallel_downloads < 1 {
                thread::available_parallelism().map_or_else(|_| 1, |c| cmp::max(c.get() - 1, 1))
            } else {
                max_parallel_downloads
            }
        }
        None => thread::available_parallelism().map_or_else(|_| 1, |c| cmp::max(c.get() - 1, 1)),
    };
    let parallel_download_lock = Arc::new(Semaphore::new(max_parallel_download));

    for url in config.blocklist_urls.iter() {
        let thread_sender = node_sender.clone();
        let thread_url = url.to_string();
        let allowed_domains = config.allowed_domains.clone();
        let thread_lock = parallel_download_lock.clone();

        thread::spawn(move || {
            let _lock_guard = thread_lock.acquire();
            info!("Starting download for URL {thread_url}");

            match run_download(&thread_url, thread_sender, allowed_domains) {
                Ok(()) => info!("Finished download for URL {thread_url}"),
                Err(e) => {
                    if e.is_write_error() {
                        error!("download failed for {thread_url}: sending blocklist to mainthread failed");
                    } else {
                        error!("download failed for {thread_url}: {e}");
                    }
                }
            };
        });
    }
    drop(node_sender);

    for domain in config.blocked_domains.iter() {
        if let Some(parts) = parse_url_to_parts(domain.as_bytes(), &config.allowed_domains) {
            blocklist.add_domain(parts);
        }
    }

    while let Ok(block_list_node) = node_receiver.recv() {
        blocklist.integrate(block_list_node);
    }

    blocklist
}

/// Notify unbound about the changed config file. This uses `unbound-control` to
/// notify, relying on unbound native communication.
fn notify_unbound() {
    let child = Command::new("unbound-control")
        .arg("reload")
        .stdout(Stdio::null())
        .spawn()
        .and_then(|mut c| c.wait());

    match child {
        Ok(exit_code) => {
            if exit_code.success() {
                info!("successfully reloaded unbound");
            } else {
                error!("error reloading unbound: exit-code {exit_code}");
            }
        }
        Err(e) => error!("error reloading unbound: {e}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    LocalLogger::setup()?;

    let config_filepath = env::args()
        .nth(1)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "usage: unbound-blocklist-generator <config.toml>"))?;

    let config = Config::new(&config_filepath)?;
    let blocklist = run_blocklist_generation(&config);

    if blocklist.is_empty() {
        warn!("no valid domains added to the blocklist, this will result in an empty file");
    }

    let output_file = File::create(config.target_filename)?;
    let mut output_stream = BufWriter::new(output_file);
    let mut shared_labels = Vec::<&[u8]>::with_capacity(8);
    output_stream.write_all("server:\n".as_bytes())?;
    blocklist.write_to(&mut output_stream, &mut shared_labels)?;
    output_stream.flush()?;

    notify_unbound();

    Ok(())
}
