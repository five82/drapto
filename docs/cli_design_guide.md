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
   - Formatting: Bold, uppercase, with dashes, full cyan color
   - Example: `----- VIDEO ANALYSIS -----`

2. **Secondary (Level 2)**: Logical groupings, operations, or completion messages
   - Formatting: Bold with leading symbol (» for operations, ✓ for success)
   - Examples: `  » Detecting black bars`, `  ✓ Analysis complete`

3. **Tertiary (Level 3)**: Individual actions or progress items
   - Formatting: Regular with progress symbol
   - Example: `    ◆ Processing encoding step`

4. **Quaternary (Level 4)**: Key-value pairs and primary information
   - Formatting: Regular text (bold values only for critical information)
   - Example: `      Input file:      movie.mkv`

5. **Supporting (Level 5)**: Details, metrics, and secondary information
   - Formatting: Regular or dimmed text
   - Example: `        Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30`

#### Whitespace Strategy

Whitespace is a critical component of visual hierarchy. Use it consistently:

- **Between all sections and subsections**: Single line break (standardized)
- **Between related items**: No line break
- **Indentation**: 2 spaces per level of hierarchy
- **Logical grouping**: Single blank line to separate logical groups

```
----- SECTION -----

  » Subsection One
    Operation in progress...
    ✓ Operation complete
      Key:              Value
      Another key:      Value

  » Subsection Two
    Another operation...
```

#### Visual Hierarchy Implementation Matrix

| Level | Element Type | Formatting | Color | Indentation | Symbol/Prefix | Example |
|-------|--------------|------------|-------|-------------|---------------|---------|
| -1 | Hardware Headers | Bold, uppercase | Blue (full) | None | ━━━━━ | `━━━━━ HARDWARE ━━━━━` |
| 0 | Batch Headers | Bold, uppercase | Yellow (full) | None | ┌─────┐ | `┌───── BATCH ENCODING ─────┐` |
| 1 | Main Sections | Bold, uppercase | Cyan (full) | None | ----- | `----- VIDEO ANALYSIS -----` |
| 2 | Subsections/Success | Bold | White | 2 spaces | » / ✓ | `  » Detecting black bars` / `  ✓ Analysis complete` |
| 3 | Operations/Progress | Regular | White | 4 spaces | Text prefix | `    Processing...` / `    Progress: 45%` |
| 4 | Primary Info | Regular | White/Green* | 6 spaces | None | `      Reduction:      65.2%` |
| 5 | Details | Regular | White/Gray | 8 spaces | None | `        Speed: 2.5x, Avg FPS: 24.5` |
| X | Critical Alert | Bold | Red/Yellow | Same as context | ✗ / ⚠ | `  ✗ Error: Encoding failed` |

*Green for significant values (>50% reductions, optimal selections, good performance)

### Color Usage

Colors should be used sparingly and meaningfully to highlight important information and establish visual hierarchy. Excessive use of color reduces its effectiveness as a tool for emphasis.

#### Strategic Color Application

- **Reserve color for emphasis** - Not every element needs color
- **Prioritize readability** - Use color to enhance, not distract from, the content
- **Maintain consistency** - Use the same color for the same type of information
- **Ensure accessibility** - All information must be accessible without color

#### Color Palette and Usage Guidelines

When colors are used, they should follow these guidelines:

#### Structural Colors (Headers and Layout)

##### Blue (Hardware Information Headers)
Use for hardware and system information section headers:
- Hardware information section headers
- System specifications sections

```
━━━━━ HARDWARE ━━━━━  ← Entire header in blue
```

##### Cyan (Primary Section Headers)
Use for major workflow phase headers:
- Section headers (entire header including delimiters)
- Major phase transitions within a file

```
----- VIDEO ANALYSIS -----  ← Entire header in cyan
```

##### Yellow (Batch Operation Headers)
Use for batch-level operation headers:
- Batch operation headers (BATCH ENCODING, BATCH COMPLETE)
- Multi-file operation indicators

```
┌───── BATCH ENCODING ─────┐  ← Entire header in yellow
```

##### Magenta (File Progress Headers)
Use for individual file progress within batch operations:
- File progress markers in batch processing
- Progress indicators between files

```
────▶ FILE 1 OF 3 ────  ← Entire header in magenta
```

#### Status and Performance Colors

##### Green (Success and Excellent Performance)
Use for positive outcomes and excellent performance:
- Major milestone completion checkmarks
- Significant file size reductions (≥50%)
- Excellent encoding speed (≥2.0x)
- Success completion messages

```
✓ Encoding complete     ← Green checkmark (major milestone)
  Reduction:    67.9%   ← Green value (significant reduction)
  Speed: 2.4x           ← Green value (excellent performance)
```

##### Yellow (Warnings and Poor Performance)
Use for warnings and performance issues:
- Poor encoding speed (≤0.2x)
- Disappointing file size reductions (≤30%)
- Warning messages and cautions
- Performance metrics requiring attention

```
⚠ Encoding speed degraded  ← Yellow warning
  Speed: 0.1x              ← Yellow value (very slow)
  Reduction: 15.3%         ← Yellow value (disappointing)
```

##### Red (Critical Errors Only)
Reserve exclusively for error conditions:
- Error messages and titles
- Failed operations
- Critical failures

```
✗ Error: Encoding failed  ← Red error message
```

#### Contextual Information Colors

##### Blue (Technical Information)
Use for technical specifications and properties:
- Video/audio format specifications
- Resolution categories (HD/UHD)
- Dynamic range indicators (HDR/SDR)
- Codec information (AV1, Opus)
- Color spaces and pixel formats

```
  Resolution:       1920x1080 (HD)    ← Blue (technical spec)
  Dynamic range:    HDR                ← Blue (technical spec)
  Audio:            5.1 surround       ← Blue (technical spec)
  Color Space:      bt709              ← Blue (technical spec)
  Pixel Format:     yuv420p10le        ← Blue (technical spec)
```

##### Light Blue (Encoder Settings)
Use for encoding configuration parameters:
- Encoder selection (SVT-AV1, Opus)
- Quality settings (CRF, presets)
- Bitrate configurations
- Encoding method choices

```
  Encoder:          SVT-AV1            ← Light blue (encoder setting)
  Preset:           6                  ← Light blue (encoder setting)
  Quality:          CRF 27             ← Light blue (encoder setting)
  Audio codec:      Opus               ← Light blue (encoder setting)
```

##### Purple (Applied Processing)
Use for active content processing settings:
- Denoising parameters and filters
- Film grain synthesis settings
- Content enhancement filters
- Applied processing indicators

```
  Denoising:        hqdn3d=2:1.5:3:2.5    ← Purple (applied processing)
  Film grain:       Level 4 (synthesis)   ← Purple (applied processing)
```

#### Neutral and De-emphasis Colors

##### White/Default (Standard Information)
Use for most text to maintain readability:
- Labels and descriptions
- Standard values and measurements
- Regular informational content
- Non-critical data

##### Gray/Dim (Minor Status and De-emphasis)
Use for less important information:
- Minor status completions (crop detection)
- Debug output in verbose mode
- Supplementary details
- Background information

```
✓ Crop detection complete  ← Dimmed checkmark (minor status)
```

##### Bold (Emphasis without Color)
Use bold formatting for emphasis when color isn't appropriate:
- Important values that don't fit color categories
- Headers and subsection titles
- Key metrics in monochrome mode

#### Practical Color Usage Examples

##### Example 1: Status Significance Indicators
```
# Good - Color indicates milestone importance
✓ Encoding complete               ← Green checkmark (major milestone)
✓ Successfully encoded 2 files    ← Green checkmark (major milestone)
✓ Crop detection complete         ← Dimmed checkmark (minor status)

# Poor - All status looks equally important
✓ Encoding complete               ← All same color/emphasis
✓ Successfully encoded 2 files
✓ Crop detection complete
```

##### Example 2: Performance-Based Color Coding
```
# Good - Color indicates performance quality
  Speed: 2.5x      ← Green (excellent: ≥2.0x)
  Speed: 1.0x      ← White (acceptable: 0.2x-2.0x)
  Speed: 0.1x      ← Yellow (concerning: ≤0.2x)

# Reduction percentages
  Reduction: 67.9% ← Green (significant: ≥50%)
  Reduction: 37.7% ← White (modest: 31-49%)
  Reduction: 15.3% ← Yellow (disappointing: ≤30%)

# Poor - No indication of performance quality
  Speed: 2.5x      ← All speeds look the same
  Speed: 1.0x
  Speed: 0.1x
```

##### Example 3: Contextual Value Emphasis
```
# Good - Color indicates information type
----- VIDEO DETAILS -----
  Resolution:       1920x1080 (HD)    ← Blue (technical information)
  Dynamic range:    HDR                ← Blue (technical information)
  Audio:            5.1 surround       ← Blue (technical information)

----- ENCODING CONFIGURATION -----
  Video:
    Encoder:        SVT-AV1            ← Light blue (encoder setting)
    Preset:         6                  ← Light blue (encoder setting)
    Quality:        CRF 27             ← Light blue (encoder setting)
    Denoising:      hqdn3d=2:1.5:3:2.5 ← Purple (applied processing)
    Film grain:     Level 4 (synthesis) ← Purple (applied processing)
  
  Advanced:
    Pixel Format:   yuv420p10le        ← Blue (technical information)
    Color Space:    bt709              ← Blue (technical information)

# Poor - No contextual distinction
  Resolution:       1920x1080 (HD)    ← All values look the same
  Encoder:          SVT-AV1
  Denoising:        hqdn3d=2:1.5:3:2.5
  Color Space:      bt709
```

#### Complete Color System Summary

**Structural Colors (Headers):**
- 🔵 Blue: Hardware information headers
- 🩵 Cyan: Primary section headers  
- 🟡 Yellow: Batch operation headers
- 🟣 Magenta: File progress headers

**Status and Performance Colors:**
- 🟢 Green: Success, major milestones, excellent performance (≥2.0x speed, ≥50% reduction)
- 🟡 Yellow: Warnings, poor performance (≤0.2x speed, ≤30% reduction)
- 🔴 Red: Critical errors and failures
- ⚫ Gray/Dim: Minor status updates, de-emphasized information

**Contextual Information Colors:**
- 🔵 Blue: Technical specifications (resolution, dynamic range, codecs, formats)
- 🩵 Light Blue: Encoder settings (SVT-AV1, presets, quality, codecs)
- 🟣 Purple: Applied processing (crop parameters or other filters)
- ⚪ White/Default: Standard information and labels

#### When NOT to Use Color

Avoid using color for:
- Decorative purposes without semantic meaning
- Every piece of data (causes color fatigue)
- Information that's already clear from context
- Labels and descriptions (use for values instead)
- Delimiters and separators (===, ---, etc.)

**Key Principle:** Most terminal text should remain uncolored (default terminal color), with color applied strategically to create meaningful distinctions and guide user attention to what matters most.

Each color should have consistent semantic meaning throughout the interface - users should be able to learn what each color represents and rely on that meaning across all contexts.

Icons should maintain the same color as their accompanying text to create a clean, professional, monochrome appearance that reduces visual distraction.

### Typography and Formatting

- **Bold**: Use for headers, important values, and to highlight critical information
- **Regular**: Use for most content
- **Dim**: Use for less important details or context
- **Uppercase**: Use sparingly, only for main section headers
- **Alignment**: Consistently align similar information for easy scanning

### Icons and Symbols

Use a minimal, consistent set of symbols that match their semantic meaning:

- **✓**: Success or completion (green for major milestones, dimmed for minor status)
- **✗**: Error or failure (red)
- **⚠**: Warning (yellow)

For in-progress operations, use spinners instead of static symbols. This provides better visual feedback and follows modern CLI conventions.

Note: Progress indicators and sample markers can use simple text formatting (e.g., "Sample 3/5:", "Progress:") instead of dedicated symbols to reduce visual complexity.

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

## Logging and Output Control

Drapto uses Rust's standard logging levels to control output verbosity, keeping the interface simple and well-understood:

### Logging Levels

The application uses standard Rust log levels controlled by the `RUST_LOG` environment variable:

- **Error**: Critical failures only
- **Warn**: Warnings and errors
- **Info** (default): Normal operation output
- **Debug**: Detailed technical information (enabled with `--verbose`)
- **Trace**: Very detailed debugging information

### Command-Line Flags

```bash
# Default output (info level)
drapto encode -i input.mkv -o output/

# Verbose output (debug level)
drapto encode --verbose -i input.mkv -o output/

# Future: Quiet mode (warn level)
# drapto encode --quiet -i input.mkv -o output/
```

### Output Examples by Level

#### Standard Output (Info Level - Default)
Shows essential information for normal operation:

```
----- VIDEO ANALYSIS -----

  » Detecting black bars
    Progress: 100.0% [##############################] (00:00:10 / 00:00:10)

  ✓ Analysis complete
    Crop detected: None required
    Processing: Encoding preset 6 (no crop adjustments required)
```

#### Verbose Output (Debug Level)
Includes technical details for troubleshooting:

```
----- VIDEO ANALYSIS -----

  » Detecting black bars
    Progress: 100.0% [##############################] (00:00:10 / 00:00:10)

[debug] Crop detection threshold: 0.1
[debug] Black border detected: none
  ✓ Analysis complete
    Crop detected: None required
    Processing: Encoding preset 6 (SVT-AV1)
```

### Design Philosophy

This approach balances simplicity with functionality:

1. **Standard conventions**: Uses familiar Rust logging levels
2. **Minimal flags**: One verbosity flag keeps the interface simple
3. **Progressive disclosure**: Debug information is available when needed
4. **Clean output**: Visual hierarchy and formatting remain consistent across all levels

The visual design principles (hierarchy, color usage, symbols) apply regardless of the logging level, ensuring a consistent and professional appearance.

## Terminal Components

### Sections

Sections create visual separation between different parts of the output:

```
----- SECTION TITLE -----

  Content goes here with consistent padding
  More content...

----- NEXT SECTION -----
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
Encoding: 45.2% [#########.......] (00:01:23 / 00:03:45)
  Speed: 2.5x, ETA: 00:02:22
```

For constrained terminal widths, adapt appropriately:
```
Encoding: 45.2% [###..]
  ETA: 00:02:22
```

### Spinners

For fast operations (typically under 5 seconds), use spinners instead of progress bars:

- Use Braille Unicode patterns for smooth animation
- Position at subsection level (2 spaces indentation)
- Clear message describing the operation
- Automatically disappear when operation completes

**Recommended spinner pattern**: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (Classic smooth rotation)

```
  ⠋ Detecting black bars...   ← Animates through smooth rotation
  ⠙ Detecting black bars...   
  ⠹ Detecting black bars...
  
  ✓ Crop detection complete   ← Replaces spinner when done
    Detected crop: crop=1920:1036:0:22
```

**Spinner characteristics**:
- 10-frame animation cycle for ultra-smooth motion
- 120ms tick interval (optimized for SSH and remote connections)
- Single-dot progression between frames (no visual jumps)
- Circular clockwise rotation pattern
- Industry standard used by npm, yarn, and other major CLI tools

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
Frames: 66,574/147,285 │ Speed: 2.5x │ FPS: 24.5 │ ETA: 00:22:30
```

## Interaction Patterns

### Command Structure

- Use consistent command structure: `drapto [global options] command [command options] [arguments]`
- Use clear, descriptive command names (e.g., `encode`)
- Keep command structure simple and intuitive

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
⧖ Detecting black bars...

# Non-interactive mode (e.g., when piped to a file)
[INFO] Detecting black bars...
```

### Progressive Disclosure

Use Rust's standard logging levels to progressively reveal technical details:

```
# Standard output (info level - default)
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

# Verbose output (debug level - with --verbose)
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

[debug] Filter chain:    hqdn3d=3.5:3.5:4.5:4.5
[debug] Encoder params:  preset=6, crf=27
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
----- FFMPEG COMMAND -----

ffmpeg
  -hwaccel videotoolbox -hwaccel_output_format nv12
  -i movie.mkv
  -c:v libsvtav1 -preset 6 -crf 27 -g 240 -pix_fmt yuv420p10le
  -vf crop=1920:1036:0:22
  -c:a libopus -b:a 128k -ac 6 -ar 48000
  -movflags +faststart
  -y movie.av1.mp4
```

### Processing Configuration Output

- Show applied crop settings and encoder parameters
- Highlight the applied configuration
- Include brief explanation of the balanced defaults

```
----- PROCESSING CONFIGURATION -----

✓ Configuration applied

  Crop Filter:           crop=1920:1036:0:22 (applied)
  Encoder Preset:        6 (balanced)
  Quality:               CRF 27 (HD profile)
  Estimated Size:        1.24 GB
  Estimated Savings:     65% vs. no processing

  Explanation: Balanced defaults (preset 6 / CRF 27) target visually transparent encodes while keeping files compact.
```

### Encoding Progress

- Show detailed progress with time estimates
- Include speed, FPS, and other relevant metrics
- Update at reasonable intervals (not too frequent)
- Provide summary upon completion

```
----- ENCODING PROGRESS -----

⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, Avg FPS: 24.5, ETA: 00:22:30

  Pass:              1/1
  Frames:            66,574 / 147,285
  Bitrate:           1,245 kb/s
  Size:              562.4 MB (current)

  Press Ctrl+C to cancel encoding
```

## Scriptability

- Ensure all output is grep-friendly
- Future: Provide quiet mode with `-q` or `--quiet` flag
- Exit with appropriate status codes
- Consider machine-readable output formats for future implementation

## Accessibility Considerations

- Support disabling color with `--no-color` flag or `NO_COLOR` environment variable
- Ensure all information is conveyed through text, not just color
- Provide verbose mode for additional context
- Support different terminal sizes and capabilities
- Ensure readability in both light and dark terminal themes

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
- No color mode (`NO_COLOR=1` environment variable)

### Test Case Example

For each combination, verify these aspects:
1. All information is accessible
2. Visual hierarchy is maintained
3. Critical information stands out
4. Text remains readable
5. Alignment is preserved when possible

Document any adjustments made for specific combinations.

## Implementation Guidelines

### Template-Based Architecture

Drapto uses a **template-based formatting system** to ensure consistent visual hierarchy and spacing across all terminal output. This approach eliminates formatting bugs and provides maintainable, predictable output.

#### Core Principles

1. **Centralized Formatting**: All output patterns are defined once as templates
2. **Data-Driven Display**: Templates receive structured data, not formatting instructions
3. **Consistent Spacing**: Templates handle all spacing and indentation automatically
4. **Type Safety**: Each template expects specific data structures

#### Template System Structure

```rust
// Template definition
pub enum TemplateData<'a> {
    KeyValueList {
        title: &'a str,
        items: Vec<(&'a str, &'a str)>,
    },
    GroupedKeyValues {
        title: &'a str,
        groups: Vec<GroupData<'a>>,
    },
    // ... other templates
}

// Usage
templates::render(TemplateData::KeyValueList {
    title: "INITIALIZATION",
    items: vec![("Input file", "movie.mkv"), ("Duration", "01:42:35")],
});
```

#### Available Templates

1. **`SectionHeader`**: Main section headers with consistent spacing
2. **`KeyValueList`**: Simple key-value pairs under a section
3. **`GroupedKeyValues`**: Key-values organized into named groups
4. **`ProgressBar`**: Progress indication with details
5. **`SpinnerToResults`**: Spinner that transitions to results
6. **`CompletionSummary`**: Success message with grouped results

#### Template Formatting Rules

- **Section headers**: Always include leading blank line, trailing blank line
- **Groups**: Separated by single blank lines
- **Key-value alignment**: Consistent 18-character label width
- **Emphasis**: Green bold for significant values (>50% reductions)
- **Indentation**: 2 spaces for all content under sections

#### Implementation Benefits

- **No manual spacing**: Templates handle all whitespace automatically
- **Consistent output**: Same template = identical formatting everywhere
- **Easy maintenance**: Fix formatting in one place, applies everywhere
- **Future-proof**: Easy to add JSON output, different terminal widths, etc.
- **Testable**: Template output is deterministic and verifiable

#### Migration from Legacy System

The template system replaces the previous `TerminalPresenter` with individual methods:

```rust
// Old approach (error-prone)
presenter.section("TITLE");
presenter.subsection("...");
presenter.status("...");

// New approach (bulletproof)
templates::render(TemplateData::KeyValueList {
    title: "TITLE",
    items: vec![("Key", "Value")],
});
```

## Command Line Arguments

Drapto follows these conventions for command line arguments:

- **Short flags**: Single-letter flags prefixed with a single dash (`-v`)
- **Long flags**: Full word flags prefixed with double dash (`--verbose`)
- **Arguments**: Values that follow flags (`--output video.mp4`)
- **Explicit flags preferred**: Use flags for clarity (`-i input.mkv` rather than positional arguments)

### Standard Flags

| Short | Long | Description |
|-------|------|-------------|
| `-h` | `--help` | Show help text |
| `-v` | `--verbose` | Enable debug-level output |
| `-V` | `--version` | Show version information |
| `-i` | `--input` | Specify input file |
| `-o` | `--output` | Specify output file |
| | `--no-color` | Disable colored output |
| | `--progress-json` | Emit structured progress for automation |

Future flags that may be added:
- `-q` / `--quiet`: Show only warnings and errors
- `--width`: Override detected terminal width

### Special Conventions

- **Stdin/Stdout**: Support `-` as a filename to read from stdin or write to stdout
  ```bash
  # Read from stdin
  cat video.mkv | drapto encode -i - -o output.mp4
  
  # Write to stdout
  drapto encode -i input.mkv -o - | another-command
  ```

## Help

- Help should be available via `drapto --help` and `drapto command --help`
- Each command should have a concise description, usage information, and examples
- Help text should follow the same visual hierarchy principles
- Include 1-2 practical example invocations in help text

## Configuration (Future)

When configuration support is added, follow this precedence order:

1. **Command-line flags** (highest priority)
2. **Environment variables** (e.g., `DRAPTO_OUTPUT_DIR`)
3. **Project-level configuration** (e.g., `.drapto.toml` in project directory)
4. **User-level configuration** (e.g., `~/.config/drapto/config.toml`)
5. **System-wide configuration** (lowest priority)

Configuration files should:
- Use TOML format for human readability
- Follow XDG Base Directory specification
- Be optional - the tool should work without any configuration

## Responsive Feedback

Following the "responsive is more important than fast" principle:

- **Show something within 100ms** - Even just "Starting..." is better than silence
- **Use progressive detail** for long operations (see Progress Feedback section)
- **Validate input early** and fail fast with helpful messages
- **Print status before heavy operations** so users know the tool is working

```
# Good - Immediate feedback
$ drapto encode -i large_file.mkv -o output/
» Initializing encoder...  ← Appears immediately
» Analyzing video properties...  ← Updates as work progresses

# Bad - Silent delay
$ drapto encode -i large_file.mkv -o output/
[5 second delay with no output]
----- INITIALIZATION -----  ← User wonders if it's working
```

## Anti-Patterns to Avoid

Based on CLI best practices, avoid these patterns:

1. **Don't require interactive prompts** - All functionality should be accessible via flags
2. **Don't create time bombs** - Avoid hard dependencies on external services
3. **Don't abbreviate subcommands** - Be explicit rather than clever
4. **Don't output developer-centric information by default** - Use debug mode for internals
5. **Don't ignore TTY detection** - Adapt output for piping vs. interactive use
6. **Don't overuse color** - Use it intentionally for emphasis, not decoration

## Detailed Examples

This section provides comprehensive examples of proper terminal output following the Drapto CLI design principles.

### Complete Workflow Example

Below is an example of a complete workflow showing the proper terminal output for a video encoding process:

```
$ drapto encode -i movie.mkv -o output_dir/

━━━━━ HARDWARE ━━━━━

  Hostname:          my-computer
  CPU:               Intel Core i7-9750H
  Memory:            16 GB
  Decoder:           VideoToolbox

----- VIDEO DETAILS -----

  File:              movie.mkv
  Duration:          01:42:35
  Resolution:        1920x1080 (HD)
  Dynamic range:     SDR
  Audio:             5.1 surround

----- VIDEO ANALYSIS -----

  ⠋ Detecting black bars...
  
  ✓ Crop detection complete
  Detected crop:     None required

----- ENCODING CONFIGURATION -----

  Video:
    Preset:          SVT-AV1 preset 6
    Quality:         CRF 27
    Denoising:       VeryLight (hqdn3d=0.5:0.4:2:2)
    Film Grain Synth: Level 4

  Advanced:
    Pixel Format:    yuv420p10le
    Color Space:     bt709

----- ENCODING PROGRESS -----

  Encoding: 45% [##########....................] (00:46:23 / 01:42:35)
  Speed: 2.5x, ETA: 00:22:30

----- ENCODING COMPLETE -----

  ✓ Encoding finished successfully

  Input file:        movie.mkv
  Output file:       movie.av1.mp4
  Original size:     3.56 GB
  Encoded size:      1.24 GB
  Reduction:         65.2%

  Video stream:      AV1 (libsvtav1), 1920x1080
  Audio stream:      Opus, 5.1 channels, 128 kb/s

  Total time:        00:40:12
  Average speed:     2.55x

  The encoded file is ready at: /home/user/videos/movie.av1.mp4
```

### Processing Configuration Detail Example

```
----- PROCESSING CONFIGURATION -----

  » Applying encoding configuration

    ◆ Crop Filter: crop=1920:1036:0:22
    ◆ Encoder Preset: 6 (balanced)
    ◆ Quality: CRF 27 (HD profile)

  ✓ Configuration applied

    Processing Settings:
      Crop Filter:        crop=1920:1036:0:22 (applied)
      Encoder Preset:     6 (balanced)
      Quality:            CRF 27 (HD profile)
      Size Reduction:     Estimated 60–65%

    Technical Details:
      Pixel format:       yuv420p10le
      Matrix coefficients: bt709
      Approach:            Conservative for quality preservation
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

### Logging Level Examples

#### Standard Output (Info Level - Default)

```
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

  Video stream:    AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:    Opus, 5.1 channels, 128 kb/s

  Total time:      00:40:12
  Average speed:   2.55x
```

#### Verbose Output (Debug Level - with --verbose flag)

```
✓ Encoding complete
  Input:           movie.mkv (3.56 GB)
  Output:          movie.av1.mp4 (1.24 GB)
  Reduction:       65.2%

  Video stream:    AV1 (libsvtav1), 1920x1080, 1,145 kb/s
  Audio stream:    Opus, 5.1 channels, 128 kb/s

  Total time:      00:40:12
  Average speed:   2.55x

[debug] Encoder:         libsvtav1 (SVT-AV1 v1.2.1)
[debug] Filter chain:    hqdn3d=3.5:3.5:4.5:4.5
[debug] Pixel format:    yuv420p10le
[debug] Color space:     bt709
[debug] Peak memory:     2.3 GB
[debug] Temp files cleaned: 5 (562.4 MB)
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
» Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, ETA: 00:22:30
```

### Interactive vs. Non-Interactive Examples

```
# Interactive mode (with spinner animation)
⧖ Detecting black bars...

# Non-interactive mode (e.g., when piped to a file)
[INFO] Detecting black bars...
```

### Width-Responsive Examples

```
# Wide terminal (120+ columns)
⧖ Encoding: 45.2% [###########.................................] (00:46:23 / 01:42:35)
  Speed: 2.5x, ETA: 00:22:30

# Standard terminal (80 columns)
⧖ Encoding: 45.2% [##########.................] (00:46:23 / 01:42:35)
  Speed: 2.5x, ETA: 00:22:30

# Narrow terminal (60 columns or less)
⧖ Encoding: 45.2% [######.....]
  ETA: 00:22:30
```

### Entry and Exit Point Examples

```
# Entry point (clear intent)
» Applying processing configuration...

# Progress indicators (clear status)
⧖ Applying encoding parameters... (80% complete)

# Exit point (clear result and next steps)
✓ Configuration applied: Preset 6, CRF 27 (crop=none)
  Next: Beginning encoding with configured settings
```

### Batch Processing Output

When processing multiple files, Drapto provides additional context and summary information using a distinct visual hierarchy:

#### Batch Initialization

```
┌───── BATCH ENCODING ─────┐  ← Yellow batch header

  Processing 3 files:
    1. movie1.mkv
    2. movie2.mkv
    3. movie3.mkv

  Output directory: /home/user/videos/output
```

#### File Progress Headers

Between each file in a batch, a distinct progress header is shown:

```
────▶ FILE 1 OF 3 ────  ← Magenta progress header

----- INITIALIZATION -----  ← Cyan section header
...
```

The visual hierarchy uses three distinct header styles:
- **Batch headers** (yellow, simple box `┌─────┐`): Highest level for batch operations
- **File progress headers** (magenta, dashes with arrow `────▶`): Mid-level progress indicators
- **Section headers** (cyan, dashes `-----`): Standard workflow phases within files

Each header type uses consistent coloring (entire header, not just text) to create clear visual separation.

#### Batch Completion Summary

```
┌───── BATCH COMPLETE ─────┐  ← Yellow batch header

  ✓ Successfully encoded 3 files

  Total original size:   10.68 GB
  Total encoded size:    3.72 GB
  Total reduction:       65.2%
  Total encoding time:   02:05:33
  Average speed:         2.45x

  Files processed:
    ✓ movie1.mkv (67.3% reduction)
    ✓ movie2.mkv (62.8% reduction)
    ✓ movie3.mkv (65.5% reduction)
```

Single file operations do not show batch headers, maintaining the existing clean output for individual encodes.
