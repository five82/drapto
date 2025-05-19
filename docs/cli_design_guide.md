# Drapto CLI Design Guide

## Introduction

This guide establishes the design principles and standards for the Drapto command-line interface (CLI). It serves as the definitive reference for creating a consistent, intuitive, and delightful user experience across all terminal interactions with Drapto.

## Core Design Philosophy

Drapto's CLI follows these fundamental principles:

1. **Human-first design**: Optimize for human readability and usability while maintaining scriptability
2. **Quiet by default**: Minimize noise, emphasize important information
3. **Consistent visual hierarchy**: Clear distinction between different levels of information
4. **Strategic color use**: Use color sparingly and meaningfully to highlight important information, not for decoration
5. **Adaptive layouts**: Adjust to terminal width
6. **Progressive disclosure**: Show most important information first

## Output Design

### Visual Hierarchy

Drapto CLI uses a consistent visual hierarchy to organize information:

1. **Primary sections**: Major workflow phases (e.g., "Grain Analysis", "Encoding")
2. **Secondary sections**: Logical groupings within phases
3. **Status lines**: Individual pieces of information
4. **Progress indicators**: Real-time feedback on ongoing operations

### Color Usage

Colors should be used sparingly and meaningfully to highlight important information and establish visual hierarchy. Excessive use of color reduces its effectiveness as a tool for emphasis.

#### Strategic Color Application

- **Reserve color for emphasis** - Not every element needs color
- **Prioritize readability** - Use color to enhance, not distract from, the content
- **Maintain consistency** - Use the same color for the same type of information

#### Color Palette

When colors are used, they should follow these guidelines:

- **Cyan (Primary)**: Use only for section headers and important dividers
- **Green (Secondary)**: Reserve for success indicators and critical values
- **Yellow (Accent)**: Use only for warnings and truly important highlights
- **Red**: Errors only
- **White/Default**: Use for most general text, including labels and standard values
- **Gray**: Less important details and secondary information

Most terminal text should remain uncolored (default terminal color), with color applied selectively to guide the user's attention to what matters most.

Icons should maintain the same color as their accompanying text to create a clean, professional, monochrome appearance that reduces visual distraction.

### Typography and Formatting

- **Bold**: Use for headers, important values, and to highlight critical information
- **Regular**: Use for most content
- **Dim**: Use for less important details or context

### Icons and Symbols

Use a consistent set of monochrome symbols with the same color as the text they accompany:

- **✓**: Success or completion
- **⧖**: In-progress or waiting
- **»**: Processing step
- **◎**: Phase indicator
- **◆**: Sample indicator
- **✗**: Error or failure

Icons should not be colored differently than their accompanying text to maintain a clean, consistent appearance. This creates a more professional look and reduces visual distraction.

## Terminal Components

### Sections

Sections create visual separation between different parts of the output:

```
===== Section Title =====
  Content goes here with consistent padding
  More content...
```

### Progress Bars

Progress bars should:
- Use the `#` character for filled portions
- Include percentage, current/total values, and ETA when available
- Adapt to terminal width
- Show additional context when relevant

```
Encoding: 45.2% [#########.......] (01:23 / 03:45), Speed: 2.5x, ETA: 02:22
```

### Status Lines

Status lines display key-value information:
- Align labels consistently
- Highlight important values with bold
- Group related status lines together

```
  Input file:      video.mp4
  Output file:     video.av1.mp4
  Resolution:      1920x1080
```

### Tables

Tables should:
- Use clear column headers (bold)
- Align content appropriately (left for text, right for numbers)
- Adapt to terminal width
- Use dividers between header and content

## Interaction Patterns

### Command Structure

- Use consistent command structure: `drapto [global options] command [command options] [arguments]`
- Group related commands under topics (e.g., `grain analyze`, `encode`)
- Use verbs for commands and nouns for topics

### Error Handling

- Make errors human-readable and actionable
- Include: what happened, why it happened, and how to fix it
- Use appropriate color (red) and formatting (bold) for error headers
- Provide context and suggestions when possible

### Progress Feedback

- Always show progress for operations that take more than 2 seconds
- Include percentage, time estimates, and operation details
- Allow interruption with clear instructions (Ctrl+C)
- Show completion confirmation

## Specific UI Patterns

### FFmpeg Command Display

- Group related flags for better readability
- Highlight different components (input, video settings, audio settings)
- Show full commands in verbose mode, simplified in regular mode

### Grain Analysis Output

- Show clear comparison between grain levels
- Use visual indicators (bars) to represent relative file sizes
- Highlight the selected/optimal level
- Include brief explanation of results

### Encoding Progress

- Show detailed progress with time estimates
- Include speed, FPS, and other relevant metrics
- Update at reasonable intervals (not too frequent)
- Provide summary upon completion

## Scriptability

- Support machine-readable output with `--json` flag
- Ensure all output is grep-friendly
- Provide quiet mode with `-q` or `--quiet` flag
- Exit with appropriate status codes

## Accessibility Considerations

- Support disabling color with `--no-color` flag or `NO_COLOR` environment variable
- Ensure all information is conveyed through text, not just color
- Provide verbose mode for additional context
- Support different terminal sizes and capabilities

## Implementation Guidelines

- Use the `terminal.rs` module for all user-facing output
- Leverage the `styling.rs` module for consistent colors and formatting
- Follow the component-based approach for complex output
- Test output in various terminal sizes and environments

## Detailed Examples

This section provides comprehensive examples of proper terminal output following the Drapto CLI design principles.

### Complete Workflow Example

Below is an example of a complete workflow showing the proper terminal output for a video encoding process:

```
$ drapto encode movie.mkv -i input_dir/ -o output_dir/

===== Initialization =====

  Input file:      movie.mkv
  Output file:     movie.av1.mp4
  Duration:        01:42:35
  Resolution:      1920x1080 (HD)
  Hardware:        VideoToolbox (decode only)

===== Video Analysis =====

» Detecting black bars
  Analyzing frames...
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

✓ Crop detection complete
  Detected crop:    None required

» Analyzing grain levels
  Extracting 5 samples for analysis...

  Sample 1/5: 00:15:23
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Sample 2/5: 00:32:47
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Sample 3/5: 00:51:18
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Sample 4/5: 01:12:05
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Sample 5/5: 01:35:42
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

===== Grain Analysis Results =====

✓ Analysis complete

  Detected Grain Level:   Moderate
  Estimated Size:         1.24 GB
  Estimated Savings:      65% vs. Baseline

  Grain Level Comparison:
    Moderate (selected) 1.24 GB  #####################
    Elevated           1.35 GB  #######################
    Light              1.42 GB  ########################
    Baseline           3.56 GB  ############################################################

  Explanation: The optimal grain level provides the best balance between file size reduction and video quality.

===== Encoding Configuration =====

  Video:
    Preset:             medium (SVT-AV1 preset 6) (default)
    Quality:            27 (CRF)
    Grain Level:        Moderate (hqdn3d=3.5:3.5:4.5:4.5)
    Film Grain Synth:   Level 10 (default)

  Hardware:
    Acceleration:       VideoToolbox (decode only)

  Advanced:
    Pixel Format:       yuv420p10le (default)
    Color Space:        bt709 (default)

===== Encoding Progress =====

⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30

  Pass:              1/1
  Frames:            66,574 / 147,285
  Bitrate:           1,245 kb/s
  Size:              562.4 MB (current)

  Press Ctrl+C to cancel encoding

===== Encoding Complete =====

✓ Encoding finished successfully

  Input file:        movie.mkv
  Output file:       movie.av1.mp4
  Duration:          01:42:35
  Original size:     3.56 GB
  Encoded size:      1.24 GB
  Reduction:         65.2%

  Video stream:      AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:      Opus, 5.1 channels, 128 kb/s

  Total time:        00:40:12
  Average speed:     2.55x

  The encoded file is ready at: /home/user/videos/movie.av1.mp4
```

### Grain Analysis Detail Example

```
===== Grain Analysis Phase 1: Initial Sampling =====

» Testing baseline grain levels on 5 samples

  Sample 1/5: 00:15:23
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Results:
    Baseline:         15.2 MB
    Light:            10.8 MB
    Moderate:          8.5 MB
    Elevated:          8.2 MB
    Heavy:             8.1 MB

  Sample 2/5: 00:32:47
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Results:
    Baseline:         14.8 MB
    Light:            10.5 MB
    Moderate:          8.3 MB
    Elevated:          8.1 MB
    Heavy:             8.0 MB

  [Additional samples omitted for brevity]

===== Grain Analysis Phase 2: Refinement =====

» Testing refined grain parameters

  Testing interpolated level between Light and Moderate
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Results:
    Light-Moderate:    9.2 MB

  Testing interpolated level between Moderate and Elevated
  ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

  Results:
    Moderate-Elevated: 8.3 MB

===== Grain Analysis Results =====

✓ Analysis complete

  Detected Grain Level:   Moderate
  Estimated Size:         1.24 GB
  Estimated Savings:      65% vs. Baseline

  Grain Level Comparison:
    Moderate (selected) 1.24 GB  #####################
    Light-Moderate      1.38 GB  ######################
    Elevated            1.35 GB  #######################
    Light               1.42 GB  ########################
    Moderate-Elevated   1.30 GB  ####################
    Heavy               1.28 GB  ####################
    Baseline            3.56 GB  ############################################################

  Technical Details:
    hqdn3d filter:       3.5:3.5:4.5:4.5
    Film grain synthesis: Level 10
    Knee threshold:      0.8
```

### Error Handling Examples

#### File Not Found Error

```
✗ Error: Input file not found

  Message:  Could not open 'movie.mkv'
  Context:  The specified input file does not exist or is not accessible

  Suggestion: Check the file path and permissions, then try again
```

#### FFmpeg Dependency Error

```
✗ Error: Missing required codec

  Message:  FFmpeg is missing libsvtav1 support
  Context:  Drapto requires FFmpeg to be compiled with libsvtav1 for AV1 encoding

  Suggestion: Install FFmpeg with libsvtav1 support:
              brew install ffmpeg --with-libsvtav1    (macOS)
              apt install ffmpeg                      (Ubuntu 22.04+)

  For more information, see: https://drapto.example.com/docs/installation
```

#### Encoding Error with Context

```
✗ Error: Encoding failed

  Message:  FFmpeg process exited with code 1
  Context:  The encoding process was interrupted at 45.2% (00:46:23 / 01:42:35)

  FFmpeg error:
    [libsvtav1 @ 0x7f8c3d0] Error code: -22
    Error initializing output stream 0:0 -- Error while opening encoder for output stream #0:0

  Suggestion: Try using a different preset or check system resources
              Run with --verbose for more detailed FFmpeg output
```

### Progress Indicators

#### Simple Progress Bar

```
⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
```

#### Detailed Progress with Multiple Metrics

```
⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30
  Pass: 1/1, Frames: 66,574 / 147,285, Bitrate: 1,245 kb/s
```

#### Multi-Stage Progress

```
» Stage 1/3: Video analysis
  ⧖ Progress: 100.0% [##############################] Complete

» Stage 2/3: Audio encoding
  ⧖ Progress: 100.0% [##############################] Complete

» Stage 3/3: Video encoding
  ⧖ Progress: 45.2% [##########.................] (00:46:23 / 01:42:35)
```

### Status Output Examples

#### Configuration Summary

```
===== Encoding Configuration =====

  Preset:             medium (SVT-AV1 preset 6)
  Quality:            27 (CRF)
  Grain Level:        Moderate (hqdn3d=3.5:3.5:4.5:4.5)
  Film Grain Synth:   Level 10
  Hardware Accel:     VideoToolbox (decode only)

  Input:              movie.mkv (1920x1080, 01:42:35)
  Output:             movie.av1.mp4

  Video settings:
    Codec:            libsvtav1
    Resolution:       1920x1080 (no scaling)
    Frame rate:       Source (23.976 fps)
    Pixel format:     yuv420p10le
    Color space:      bt709

  Audio settings:
    Codec:            libopus
    Channels:         5.1
    Sample rate:      48000 Hz
    Bitrate:          128 kb/s
```

#### FFmpeg Command Display

```
===== FFmpeg Command =====

ffmpeg
  -hwaccel videotoolbox -hwaccel_output_format nv12
  -i movie.mkv
  -c:v libsvtav1 -preset 6 -crf 27 -g 240 -pix_fmt yuv420p10le
  -svtav1-params film-grain=10
  -vf hqdn3d=3.5:3.5:4.5:4.5
  -c:a libopus -b:a 128k -ac 6 -ar 48000
  -movflags +faststart
  -y movie.av1.mp4
```

#### Completion Summary

```
===== Encoding Complete =====

✓ Encoding finished successfully

  Input file:        movie.mkv
  Output file:       movie.av1.mp4
  Duration:          01:42:35
  Original size:     3.56 GB
  Encoded size:      1.24 GB
  Reduction:         65.2%

  Video stream:      AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:      Opus, 5.1 channels, 128 kb/s

  Total time:        00:40:12
  Average speed:     2.55x

  The encoded file is ready at: /home/user/videos/movie.av1.mp4
```

## Command Line Arguments

Drapto follows these conventions for command line arguments:

- **Short flags**: Single-letter flags prefixed with a single dash (`-v`)
- **Long flags**: Full word flags prefixed with double dash (`--verbose`)
- **Arguments**: Values that follow flags (`--output video.mp4`)
- **Positional arguments**: Required values without flags (`drapto encode video.mp4`)

### Standard Flags

| Short | Long | Description |
|-------|------|-------------|
| `-h` | `--help` | Show help text |
| `-v` | `--verbose` | Show detailed output |
| `-q` | `--quiet` | Suppress non-essential output |
| `-o` | `--output` | Specify output file |
| `-f` | `--force` | Force operation without confirmation |
| | `--json` | Output in JSON format |
| | `--no-color` | Disable colored output |

## Help

- Help should be available via `drapto --help` and `drapto command --help`
- Each command should have a concise description, usage information, and examples
