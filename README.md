# delenix

## An open source and cross-platform (hopefully) application for screenshotting and uploading media.

delenix aims to be a simple and easy to use application for taking screenshots and uploading them to custom destinations.

## !! THE PROJECT IS IN REALLY EARLY STAGES OF DEVELOPMENT !!

Below you can find all of the currently supported features and the ones that are planned to be implemented.
Please note that at this moment in time, Linux is the only supported platform, but Windows support is planned and partially implemented. As for MacOS... lol.

## Features

- [x] Taking screenshots (full screen, region, window is supported but not exposed to use yet)
- [x] Saving screenshots to disk
- [x] Saving screenshots to clipboard after taking them
- [ ] Daemon mode (running in the background and taking screenshots on keypress, configurable and for use with the GUI, currently still in semi-design phase)
- [x] Uploading screenshots to custom destinations (the framework and such is already implemented, but actually making the requests is not yet implemented)
- [ ] GIF recording
- [ ] MP4 recording
- [ ] GUI for configuring the application (the concept of which would be inspired by ShareX)
- [ ] Compatibility with ShareX configuration files (for easy migration)
- [x] OCR Support for text in screenshots

There are also some other features that are planned to be implemented, but are not yet listed here because they are not yet fully designed or I just forgot about them. Please feel free to open an issue if you have any suggestions or ideas, or even code improvements. I would love to see them! :)

## Building

Delenix is written in Rust, so you will need to have Rust installed in order to build it. You can get it from [here](https://rustup.rs/).

After you have Rust installed, you can clone the repository and build the project by running the following command in the root directory of the project:

```bash
cargo build --release
```

If you wish to have OCR support (which is disabled by default), you will need to install the Tesseract OCR engine and the corresponding language data. You can find instructions on how to do that [here](https://tesseract-ocr.github.io/tessdoc/Installation.html). After you have installed Tesseract, you can enable OCR support by running the following command:

```bash
cargo build --release --features tesseract
```

Note that all prebuilt binaries have OCR support enabled, however you will still need to install Tesseract and the language data in order to use it.

Upon buidling, the binary will be located in `target/release/delenix`.

## Installing

Delenix is not yet packaged for any Linux distribution, but it will be in the future. For now, you can build it yourself and copy the binary to `/usr/bin` or `/usr/local/bin` (or any other directory in your `$PATH`).

## Usage

Delenix is a command line application, so you can run it from your terminal. It has a few subcommands, which you can see by running `delenix --help`. The most important one is `delenix -s`, which takes a screenshot and does what the configuration tells it to do.

As a suggestion, you can bind the `delenix -s` command to a key combination in your window manager. For example, in xfce you can do that by going to `Settings > Keyboard > Application Shortcuts` and adding a new shortcut with the command `delenix -s`. (This is what I'm doing.)

## Configuration

Delenix is configured using a JSON file. The default configuration file is located in `~/.config/delenix/config.json`. You can also specify a custom configuration file by using the `-c` flag.