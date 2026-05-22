// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

mod gui;

use std::env;
use std::path::Path;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Parse command line arguments for initial files to open
    let mut initial_paths = Vec::new();
    let cwd = env::current_dir().unwrap_or_default();
    
    let mut parse_args = true;
    for arg in env::args_os().skip(1) {
        if parse_args && (arg == "-h" || arg == "--help") {
            print_help();
            return Ok(());
        }
        if parse_args && (arg == "-v" || arg == "--version") {
            print_version();
            return Ok(());
        }
        if parse_args && arg == "--" {
            parse_args = false;
            continue;
        }

        let path = cwd.join(Path::new(&arg));
        initial_paths.push(path);
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Edit")
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Edit",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(gui::EditApp::new(cc, initial_paths)))
        }),
    )
}

fn print_help() {
    println!(concat!(
        "Usage: edit [OPTIONS] [FILE]\n",
        "Options:\n",
        "    -h, --help       Print this help message\n",
        "    -v, --version    Print the version number\n",
        "\n",
        "Arguments:\n",
        "    FILE             The file to open\n",
    ));
}

fn print_version() {
    println!(concat!("edit version ", env!("CARGO_PKG_VERSION")));
}
