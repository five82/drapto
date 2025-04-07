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

Basic usage involves specifying an input file/directory and an output directory. By default, Drapto runs in **daemon mode**, meaning it will start the encoding process in the background and detach from the terminal, allowing you to log out while it continues running. Log files are created in the specified log directory (or `output_dir/logs` by default). A PID file (`drapto.pid`) is also created in the log directory to track the running process.

To run Drapto in the foreground (interactive mode), use the `--interactive` flag. This will display progress and logs directly in your terminal.
```bash
# Encode a single file in the background (default daemon mode)
drapto encode /path/to/input/video.mkv /path/to/output/

# Encode all videos in a directory in the background
drapto encode /path/to/input_directory/ /path/to/output_directory/

# Encode a single file interactively (in the foreground)
drapto encode --interactive /path/to/input/video.mkv /path/to/output/

# Encode and send notifications to an ntfy.sh topic
drapto encode video.mkv output/ --ntfy https://ntfy.sh/your_topic
```

### Notifications

Drapto can send notifications about encoding progress (start, success, error) to an [ntfy.sh](https://ntfy.sh/) topic URL. The notification message will include the hostname where the encode job is running.

*   Use the `--ntfy <topic_url>` argument to specify the topic URL.
*   Alternatively, set the `DRAPTO_NTFY_TOPIC` environment variable.
*   If both are set, the command-line argument takes precedence.