# Drapto

HandbrakeCLI video encoding wrapper.

Drapto provides a convenient command-line interface to automate video encoding tasks using HandBrakeCLI. It simplifies the process by allowing you to define encoding presets and apply them easily.

## Features

*   Wraps HandBrakeCLI for powerful and flexible video encoding.
*   Uses built-in default encoding settings.

## Film Grain Detection

Drapto includes a feature to help determine optimal settings for HandBrake's film grain filter (`--encoder-preset grain=<value>`). This can assist users in finding a good balance between perceived visual quality and the resulting file size.

The feature works by encoding short samples of the source video using different film grain values. After encoding, it reports the file size (in Megabytes - MB) generated for each grain setting tested. This allows for a direct comparison of how different grain levels impact the output file size.

## Installation

1.  **Install HandBrakeCLI:** Ensure you have `HandBrakeCLI` installed and available in your system's PATH. You can download it from the [official HandBrake website](https://handbrake.fr/downloads2.php).
2.  **Install Rust:** If you don't have Rust installed, follow the instructions at [rustup.rs](https://rustup.rs/).
3.  **Install Drapto:** Install directly from the Git repository using `cargo install`.
    ```bash
    cargo install --git https://github.com/five82/drapto
    ```
    This command clones the repository, builds the `drapto` binary, and installs it to `~/.cargo/bin/`.

    **Important:** Ensure `~/.cargo/bin` is included in your system's PATH environment variable so you can run `drapto` from anywhere.

## Usage

Basic usage involves specifying an input file/directory and an output directory. Drapto will use built-in default encoding settings.

```bash
# Encode a single file using default settings
drapto encode -i /path/to/input/video.mkv -o /path/to/output/

# Encode all videos in a directory
drapto encode -i /path/to/input_directory/ -o /path/to/output_directory/

```