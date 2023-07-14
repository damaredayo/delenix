use std::{thread, time};

use delenix_lib::{clipboard, config, screenshot, util};
use structopt::StructOpt;

mod ipc;

#[derive(Debug, StructOpt)]
#[structopt(name = "delenix", about = "A screenshotting and file uploading tool.")]
struct Cli {
    #[structopt(
        short = "c",
        long = "config",
        value_name = "FILE",
        help = "Specify a config file"
    )]
    config: Option<String>,

    #[structopt(short = "d", long = "daemon", help = "Start the daemon")]
    daemon: bool,

    #[structopt(
        short = "s",
        long = "screenshot",
        help = "Take a full screen screenshot"
    )]
    screenshot: bool,

    #[structopt(
        short = "u",
        long = "upload",
        help = "Upload a file, requires -f/--file"
    )]
    upload: bool,

    #[structopt(
        short = "f",
        long = "file",
        value_name = "FILE",
        help = "File to upload, to be used with -u/--upload"
    )]
    file: Option<String>,
}

fn main() {
    tracing_subscriber::fmt::init();

    let opt = Cli::from_args();

    // if no arguments are specified, show help
    if !opt.daemon && !opt.screenshot && !opt.upload && opt.file.is_none() {
        Cli::clap().print_help().expect("Failed to print help");
        return;
    }

    // check there are no conflicting arguments
    if opt.daemon && (opt.screenshot || opt.upload || opt.file.is_some()) {
        tracing::error!("Daemon mode may only be used in conjunction with the --config argument.");
        return;
    }

    let config_path = opt.config.unwrap_or_else(util::make_default_config_path);
    let config = {
        tracing::info!("Loading config from {}", config_path);
        config::Config::from_file(config_path).unwrap()
    };

    if opt.daemon {
        tracing::info!("Starting daemon");

        let rt = tokio::runtime::Runtime::new().unwrap();

        let arc_mutex_config = std::sync::Arc::new(tokio::sync::Mutex::new(config));

        rt.block_on(ipc::start_ipc(arc_mutex_config));

        return;
    }

    if opt.upload {
        if let Some(path) = opt.file {
            let data = std::fs::read(&path).unwrap();
            util::handle_simple_upload(&config, &data);
        } else {
            tracing::error!("No file specified to upload");
        }
    }

    if opt.screenshot {
        tracing::info!("Taking screenshot");
        let rs = delenix_lib::screenshot::select_region(config.freeze_screen).unwrap();
        thread::sleep(time::Duration::from_millis(30)); // this is a hack to fix the screenshot sometimes displaying the dim and selection rectangle
        let png = config
            .screenshot(screenshot::ScreenshotType::Region(rs))
            .unwrap();

        if config.copy_to_clipboard {
            clipboard::copy_png_to_clipboard(&png).unwrap();
        }

        if !config.uploaders.is_empty() {
            util::handle_simple_upload(&config, &png);
        }
    }
}
