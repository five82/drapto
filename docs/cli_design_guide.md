# Drapto CLI Design Guide

## Introduction

This guide establishes the design principles and standards for the Drapto command-line interface (CLI). It serves as the definitive reference for creating a consistent, intuitive, and delightful user experience across all terminal interactions with Drapto.

## Core Design Philosophy

Drapto's CLI follows these fundamental principles:

1. **Human-first design**: Optimize for human readability and usability while maintaining scriptability
2. **Quiet by default**: Minimize noise, emphasize important information
3. **Consistent visual hierarchy**: Clear distinction between different levels of information
4. **Strategic color use**: Use color sparingly and meaningfully to highlight important information, not for decoration
5. **Adaptive layouts**: Adjust to terminal width, capabilities, and output mode
6. **Progressive disclosure**: Show most important information first, reveal details progressively

## Output Design

### Visual Hierarchy

Drapto CLI uses a consistent and well-defined visual hierarchy to organize information:

#### Hierarchy Levels

1. **Primary (Level 1)**: Major workflow phases and main section headers
   - Formatting: Bold, uppercase, with separators
   - Example: `===== VIDEO ANALYSIS =====`

2. **Secondary (Level 2)**: Logical groupings, operations, or completion messages
   - Formatting: Bold with leading symbol (» for operations, ✓ for success)
   - Examples: `  » Analyzing grain levels`, `  ✓ Analysis complete`

3. **Tertiary (Level 3)**: Individual actions or progress items
   - Formatting: Regular with progress symbol
   - Example: `    ⧖ Processing sample 3/5`

4. **Quaternary (Level 4)**: Key-value pairs and primary information
   - Formatting: Regular text (bold values only for critical information)
   - Example: `    Input file:      movie.mkv`

5. **Supporting (Level 5)**: Details, metrics, and secondary information
   - Formatting: Regular or dimmed text
   - Example: `    Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30`

#### Whitespace Strategy

Whitespace is a critical component of visual hierarchy. Use it consistently:

- **Between major sections**: Single line break
- **Between subsections**: Single line break
- **Between related items**: No line break
- **Indentation**: 2 spaces per level of hierarchy
- **Logical grouping**: Use blank lines to separate logical groups of information

```
===== SECTION =====

  » Subsection One
    ⧖ Operation in progress
    ✓ Operation complete
      Key:              Value
      Another key:      Value

  » Subsection Two
    ⧖ Another operation
```

#### Visual Hierarchy Implementation Matrix

| Level | Element Type | Formatting | Color | Indentation | Symbol | Example |
|-------|--------------|------------|-------|-------------|--------|---------|
| 1 | Main Sections | Bold, uppercase | Cyan | None | ===== | `===== VIDEO ANALYSIS =====` |
| 2 | Subsections/Success | Bold | White | 2 spaces | » / ✓ | `  » Analyzing grain levels` / `  ✓ Analysis complete` |
| 3 | Operations/Progress | Regular | White | 4 spaces | ⧖ / ◆ | `    ⧖ Processing sample 3/5` |
| 4 | Primary Info | Regular | White | 4 spaces | None | `    Input file:      movie.mkv` |
| 5 | Details | Regular | White/Gray | 4-6 spaces | None | `    Speed: 2.5x, Avg FPS: 24.5` |
| X | Critical Alert | Bold | Red/Yellow | Same as context | ✗ / ⚠ | `  ✗ Error: Encoding failed` |

### Color Usage

Colors should be used sparingly and meaningfully to highlight important information and establish visual hierarchy. Excessive use of color reduces its effectiveness as a tool for emphasis.

#### Strategic Color Application

- **Reserve color for emphasis** - Not every element needs color
- **Prioritize readability** - Use color to enhance, not distract from, the content
- **Maintain consistency** - Use the same color for the same type of information
- **Ensure accessibility** - All information must be accessible without color

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
- **Uppercase**: Use sparingly, only for main section headers
- **Alignment**: Consistently align similar information for easy scanning

### Icons and Symbols

Use a consistent set of monochrome symbols with the same color as the text they accompany:

- **✓**: Success or completion
- **⧖**: In-progress or waiting
- **»**: Processing step or subsection
- **◎**: Phase indicator
- **◆**: Sample indicator
- **✗**: Error or failure
- **⚠**: Warning
- **ℹ**: Information

Icons should not be colored differently than their accompanying text to maintain a clean, consistent appearance. This creates a more professional look and reduces visual distraction.

### Focus Techniques

Use these techniques to draw attention to the most important information:

- **Positioning**: Place important information first in a section
- **Whitespace**: Surround important elements with whitespace
- **Bold formatting**: Use bold for key values and metrics
- **Symbols**: Precede important status updates with appropriate symbols
- **Summary highlight**: Use a summary line for key metrics
  ```
  Reduction: 65.2% (3.56 GB → 1.24 GB)  ← Important metric stands out
  ```

## Information Density Guidelines

Balance information density appropriately based on context and user needs:

### Low Density (For Critical Information)
Use for alerts, errors, and key status updates that need immediate attention.

```
✗ Error: Encoding failed
  Try using a different preset or check system resources
```

### Medium Density (For Standard Output)
Use for most terminal output where users are actively watching.

```
⧖ Encoding: 45.2% [##########.................]
  Speed: 2.5x, ETA: 00:22:30
  Pass: 1/1, Frames: 66,574 / 147,285
```

### High Density (For Detailed Analysis)
Use when users request comprehensive information, such as with `--verbose`.

```
⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30
  Pass: 1/1, Frames: 66,574 / 147,285, Bitrate: 1,245 kb/s
  Buffer: 256MB, Queue: 24 frames, GOP: 240, Ref frames: 4
  CPU: 87%, Memory: 1.2GB, Temp: 75°C, Power: Medium
```

Always allow users to control the information density with flags like `--quiet`, `--normal` (default), and `--verbose`.

## Terminal Components

### Sections

Sections create visual separation between different parts of the output:

```
===== SECTION TITLE =====
  Content goes here with consistent padding
  More content...

===== NEXT SECTION =====
```

Use a single blank line between major sections to enhance visual separation.

### Progress Bars

Progress bars should:
- Use the `#` character for filled portions
- Include percentage, current/total values, and ETA when available
- Adapt to terminal width
- Show additional context when relevant
- Update at an appropriate frequency (not too fast, not too slow)

```
Encoding: 45.2% [#########.......] (01:23 / 03:45), Speed: 2.5x, ETA: 02:22
```

For constrained terminal widths, adapt appropriately:
```
Encoding: 45.2% [###..]
```

### Status Lines

Status lines display key-value information:
- Align labels consistently
- Use bold sparingly, only for critical values (e.g., significant reductions)
- Group related status lines together
- Use consistent spacing for alignment

```
  Input file:      video.mp4
  Output file:     video.av1.mp4
  Resolution:      1920x1080
  Reduction:       65.2%
```

### Tables

Tables should:
- Use clear column headers (bold)
- Align content appropriately (left for text, right for numbers)
- Adapt to terminal width
- Use dividers between header and content
- Only be used when tabular data presentation is necessary

```
Sample  | Time     | Size (MB) | Quality | Selection
--------|----------|-----------|---------|----------
1       | 00:15:23 | 15.2      | 86.7    | Baseline
2       | 00:32:47 | 10.5      | 85.2    | Light
3       | 00:51:18 |  8.3      | 84.9    | Moderate *
4       | 01:12:05 |  8.1      | 84.1    | Elevated
5       | 01:35:42 |  8.0      | 83.6    | Heavy
```

### Composite Information Display

When displaying multiple related data points, use efficient layouts:

```
# Before: Separate lines that are hard to scan
Frame: 66,574
Total Frames: 147,285
Speed: 2.5x
FPS: 24.5
ETA: 00:22:30

# After: Organized grouping with visual separators
Frame: 66,574/147,285 │ Speed: 2.5x │ FPS: 24.5 │ ETA: 00:22:30
```

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

```
✗ Error: Input file not found

  Message:  Could not open 'movie.mkv'
  Context:  The specified input file does not exist or is not accessible

  Suggestion: Check the file path and permissions, then try again
```

### Progress Feedback

- Always show progress for operations that take more than 2 seconds
- Include percentage, time estimates, and operation details
- Allow interruption with clear instructions (Ctrl+C)
- Show completion confirmation
- Adapt detail level based on operation duration:

```
# First 0.5 seconds - Just show operation started
» Starting encoding...

# After 1-2 seconds - Show simple progress
» Encoding: [...................]

# After 5 seconds - Add percentage
» Encoding: 12% [##...............]

# After 30 seconds - Full details
» Encoding: 45.2% [##########.................]
  Speed: 2.5x, ETA: 00:22:30
```

### Entry and Exit Points

Clearly mark the beginning and end of operations:

```
# Starting an operation (clear intent)
» Starting grain analysis on 5 samples...

# Intermediate status (clear progress)
⧖ Analyzing sample 3/5... (60% complete)

# Completion (clear result and next steps)
✓ Analysis complete: Moderate grain detected
  Next: Beginning encoding with optimized settings
```

### Interactive vs. Non-Interactive Modes

Output should adapt based on the terminal environment:

```
# Interactive mode (with spinner animation)
⧖ Analyzing grain levels...

# Non-interactive mode (e.g., when piped to a file)
[INFO] Analyzing grain levels...
```

### Progressive Disclosure

Implement progressive disclosure to show the most important information first:

```
# Basic output (default)
✓ Encoding complete: 1.24 GB (65.2% reduction)

# Detailed output (-v or --verbose)
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

  Video stream:    AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:    Opus, 5.1 channels, 128 kb/s
```

### Context-Aware Displays

Output should adapt based on terminal width and capabilities:

```
# When running interactively (with full terminal)
⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30

# When terminal width is limited
⧖ Encoding: 45.2% [######.....]
  ETA: 00:22:30

# When piped to another command (detected automatically)
Encoding: 45.2%, ETA: 00:22:30
```

## Specific UI Patterns

### FFmpeg Command Display

- Group related flags for better readability
- Highlight different components (input, video settings, audio settings)
- Show full commands in verbose mode, simplified in regular mode

```
===== FFMPEG COMMAND =====

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

### Grain Analysis Output

- Show clear comparison between grain levels
- Use visual indicators (bars) to represent relative file sizes
- Highlight the selected/optimal level
- Include brief explanation of results

```
===== GRAIN ANALYSIS RESULTS =====

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
```

### Encoding Progress

- Show detailed progress with time estimates
- Include speed, FPS, and other relevant metrics
- Update at reasonable intervals (not too frequent)
- Provide summary upon completion

```
===== ENCODING PROGRESS =====

⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30

  Pass:              1/1
  Frames:            66,574 / 147,285
  Bitrate:           1,245 kb/s
  Size:              562.4 MB (current)

  Press Ctrl+C to cancel encoding
```

## Scriptability

- Support machine-readable output with `--json` flag
- Ensure all output is grep-friendly
- Provide quiet mode with `-q` or `--quiet` flag
- Exit with appropriate status codes

### JSON Output Format

```json
{
  "status": "in_progress",
  "operation": "encoding",
  "progress": {
    "percent": 45.2,
    "current_time": "00:46:23",
    "total_time": "01:42:35",
    "eta": "00:22:30",
    "speed": 2.5,
    "fps": 24.5
  },
  "details": {
    "pass": 1,
    "total_passes": 1,
    "frames_processed": 66574,
    "total_frames": 147285,
    "bitrate": 1245,
    "current_size_mb": 562.4
  }
}
```

## Accessibility Considerations

- Support disabling color with `--no-color` flag or `NO_COLOR` environment variable
- Ensure all information is conveyed through text, not just color
- Provide verbose mode for additional context
- Support different terminal sizes and capabilities
- Ensure readability in both light and dark terminal themes
- Add optional descriptions for screen readers with `--screen-reader` flag

## Terminal Testing Grid

To ensure consistent visual hierarchy across environments, test your CLI in the following scenarios:

### Terminal Types
- Modern terminals with full color support (iTerm2, Windows Terminal)
- Basic terminals with limited color (standard macOS Terminal, cmd.exe)
- Monochrome terminals (SSH sessions to some servers)
- Terminal multiplexers (tmux, screen)

### Width Scenarios
- Wide terminal (120+ columns)
- Standard terminal (80 columns)
- Narrow terminal (60 columns or less)
- Dynamic resizing (test what happens when terminal is resized during operation)

### Output Modes
- Interactive (TTY available)
- Non-interactive (output piped to file or another program)
- CI/CD environment (GitHub Actions, Jenkins)
- Remote shell environments (SSH sessions)

### Accessibility Scenarios
- High contrast mode
- Screen readers (test with descriptions)
- No color mode (`NO_COLOR=1` environment variable)
- Different color schemes (light/dark terminals)

### Test Case Example

For each combination, verify these aspects:
1. All information is accessible
2. Visual hierarchy is maintained
3. Critical information stands out
4. Text remains readable
5. Alignment is preserved when possible

Document any adjustments made for specific combinations.

## Implementation Guidelines

- Use the `terminal.rs` module for all user-facing output
- Leverage the `styling.rs` module for consistent colors and formatting
- Follow the component-based approach for complex output
- Test output in various terminal sizes and environments
- Implement responsive output algorithms:

```rust
fn adjust_output_by_width(width: u16) -> OutputDetail {
    match width {
        0..=50 => OutputDetail::Minimal,
        51..=80 => OutputDetail::Standard,
        _ => OutputDetail::Full
    }
}
```

- Create a reusable style guide library to ensure consistency
- Implement visual regression testing
- Add user feedback mechanisms

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
| | `--width` | Override detected terminal width |
| | `--screen-reader` | Enable screen reader descriptions |

## Help

- Help should be available via `drapto --help` and `drapto command --help`
- Each command should have a concise description, usage information, and examples
- Help text should follow the same visual hierarchy principles

## Detailed Examples

This section provides comprehensive examples of proper terminal output following the Drapto CLI design principles.

### Complete Workflow Example

Below is an example of a complete workflow showing the proper terminal output for a video encoding process:

```
$ drapto encode movie.mkv -i input_dir/ -o output_dir/

===== INITIALIZATION =====

  Input file:      movie.mkv
  Output file:     movie.av1.mp4
  Duration:        01:42:35
  Resolution:      1920x1080 (HD)
  Hardware:        VideoToolbox (decode only)

===== VIDEO ANALYSIS =====

  » Detecting black bars
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

===== GRAIN ANALYSIS RESULTS =====

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

===== ENCODING CONFIGURATION =====

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

===== ENCODING PROGRESS =====

  ⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
    Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30

    Pass:              1/1
    Frames:            66,574 / 147,285
    Bitrate:           1,245 kb/s
    Size:              562.4 MB (current)

    Press Ctrl+C to cancel encoding

===== ENCODING COMPLETE =====

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
===== GRAIN ANALYSIS PHASE 1: INITIAL SAMPLING =====

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

===== GRAIN ANALYSIS PHASE 2: REFINEMENT =====

  » Testing refined grain parameters

    Testing interpolated level between Light and Moderate
    ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

    Results:
      Light-Moderate:    9.2 MB

    Testing interpolated level between Moderate and Elevated
    ⧖ Progress: 100.0% [##############################] (10.0 / 10.0s)

    Results:
      Moderate-Elevated: 8.3 MB

===== GRAIN ANALYSIS RESULTS =====

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

### Progressive Disclosure Examples

#### Basic Output (Default)

```
✓ Encoding complete: 1.24 GB (65.2% reduction)
```

#### Standard Output

```
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%
```

#### Detailed Output (--verbose)

```
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

  Video stream:    AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:    Opus, 5.1 channels, 128 kb/s

  Processing:      00:40:12 at 2.55x speed
  Encoder:         libsvtav1 (SVT-AV1 v1.2.1)
  Filter chain:    hqdn3d=3.5:3.5:4.5:4.5

  The encoded file is ready at: /home/user/videos/movie.av1.mp4
```

### Visual Alignment Examples

```
# Consistent key-value alignment
  Input file:      movie.mkv         # Note the consistent spacing
  Output file:     movie.av1.mp4     # for alignment of values
  Duration:        01:42:35
  Original size:   3.56 GB

# Hierarchical indentation for nested information
  Video:
    Codec:         libsvtav1         # Nested information is consistently
    Resolution:    1920x1080         # indented with 2 spaces
    Pixel format:  yuv420p10le
```

### Time-Based Output Examples

```
# First 0.5 seconds
» Starting encoding...

# After 1-2 seconds
» Encoding: [...................]

# After 5 seconds
» Encoding: 12% [##...............]

# After 30 seconds
» Encoding: 45.2% [##########.................]
  Speed: 2.5x, ETA: 00:22:30
```

### Interactive vs. Non-Interactive Examples

```
# Interactive mode (with spinner animation)
⧖ Analyzing grain levels...

# Non-interactive mode (e.g., when piped to a file)
[INFO] Analyzing grain levels...
```

### Width-Responsive Examples

```
# Wide terminal (120+ columns)
⧖ Encoding: 45.2% [###########.................................] (00:46:23 / 01:42:35) Speed: 2.5x, ETA: 00:22:30

# Standard terminal (80 columns)
⧖ Encoding: 45.2% [##########.................]
  Speed: 2.5x, ETA: 00:22:30

# Narrow terminal (60 columns or less)
⧖ Encoding: 45.2% [######.....]
  ETA: 00:22:30
```

### Entry and Exit Point Examples

```
# Entry point (clear intent)
» Starting grain analysis on 5 samples...

# Progress indicators (clear status)
⧖ Analyzing sample 3/5... (60% complete)

# Exit point (clear result and next steps)
✓ Analysis complete: Moderate grain detected
  Next: Beginning encoding with optimized settings
```
