package reporter

import (
	"fmt"
	"os"
	"strings"
	"sync"

	"github.com/fatih/color"
	"github.com/five82/drapto/internal/util"
	"github.com/schollz/progressbar/v3"
)

// TerminalReporter outputs human-friendly text to the terminal.
type TerminalReporter struct {
	mu         sync.Mutex
	progress   *progressbar.ProgressBar
	maxPercent float32
	lastStage  string
	cyan       *color.Color
	green      *color.Color
	yellow     *color.Color
	red        *color.Color
	magenta    *color.Color
	bold       *color.Color
}

// NewTerminalReporter creates a new terminal reporter.
func NewTerminalReporter() *TerminalReporter {
	return &TerminalReporter{
		cyan:    color.New(color.FgCyan, color.Bold),
		green:   color.New(color.FgGreen),
		yellow:  color.New(color.FgYellow, color.Bold),
		red:     color.New(color.FgRed, color.Bold),
		magenta: color.New(color.FgMagenta),
		bold:    color.New(color.Bold),
	}
}

func (r *TerminalReporter) finishProgress() {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.progress != nil {
		_ = r.progress.Finish()
		r.progress = nil
	}
	r.maxPercent = 0
}

func (r *TerminalReporter) Hardware(summary HardwareSummary) {
	fmt.Println()
	_, _ = r.cyan.Println("HARDWARE")
	r.printLabel(10, "Hostname:", summary.Hostname)
}

// printLabel prints a bold label with fixed width padding followed by a value.
// Width is applied to the plain text before styling to ensure proper alignment.
func (r *TerminalReporter) printLabel(width int, label, value string) {
	paddedLabel := fmt.Sprintf("%-*s", width, label)
	fmt.Printf("  %s %s\n", r.bold.Sprint(paddedLabel), value)
}

func (r *TerminalReporter) Initialization(summary InitializationSummary) {
	fmt.Println()
	_, _ = r.cyan.Println("VIDEO")
	r.printLabel(10, "File:", summary.InputFile)
	r.printLabel(10, "Output:", summary.OutputFile)
	r.printLabel(10, "Duration:", summary.Duration)
	r.printLabel(10, "Resolution:", fmt.Sprintf("%s (%s)", summary.Resolution, summary.Category))
	r.printLabel(10, "Dynamic:", summary.DynamicRange)
	r.printLabel(10, "Audio:", summary.AudioDescription)
}

func (r *TerminalReporter) StageProgress(update StageProgress) {
	r.mu.Lock()
	if r.lastStage != update.Stage {
		r.mu.Unlock()
		fmt.Println()
		_, _ = r.cyan.Println(strings.ToUpper(update.Stage))
		r.mu.Lock()
		r.lastStage = update.Stage
	}
	r.mu.Unlock()
	fmt.Printf("  %s %s\n", r.magenta.Sprint("›"), update.Message)
}

func (r *TerminalReporter) CropResult(summary CropSummary) {
	var status string
	if summary.Disabled {
		status = color.New(color.Faint).Sprint("auto-crop disabled")
	} else if summary.Required {
		status = r.green.Sprint(summary.Crop)
	} else {
		status = color.New(color.Faint).Sprint("no crop needed")
	}
	fmt.Printf("  %s %s (%s)\n", r.bold.Sprint("Crop detection:"), summary.Message, status)
}

func (r *TerminalReporter) EncodingConfig(summary EncodingConfigSummary) {
	fmt.Println()
	_, _ = r.cyan.Println("ENCODING")
	const w = 14 // Width to fit "Drapto preset:" and "Preset values:"
	r.printLabel(w, "Encoder:", summary.Encoder)
	r.printLabel(w, "Preset:", summary.Preset)
	r.printLabel(w, "Tune:", summary.Tune)
	r.printLabel(w, "Quality:", summary.Quality)
	r.printLabel(w, "Pixel format:", summary.PixelFormat)
	r.printLabel(w, "Matrix:", summary.MatrixCoefficients)
	r.printLabel(w, "Audio codec:", summary.AudioCodec)
	r.printLabel(w, "Audio:", summary.AudioDescription)
	r.printLabel(w, "Drapto preset:", summary.DraptoPreset)

	if len(summary.DraptoPresetSettings) > 0 {
		var parts []string
		for _, kv := range summary.DraptoPresetSettings {
			parts = append(parts, fmt.Sprintf("%s=%s", kv[0], kv[1]))
		}
		r.printLabel(w, "Preset values:", strings.Join(parts, ", "))
	}

	if summary.SVTAV1Params != "" {
		r.printLabel(w, "SVT params:", summary.SVTAV1Params)
	}
}

func (r *TerminalReporter) EncodingStarted(totalFrames uint64) {
	r.finishProgress()

	r.mu.Lock()
	defer r.mu.Unlock()

	r.progress = progressbar.NewOptions64(
		100,
		progressbar.OptionSetDescription(""),
		progressbar.OptionSetWidth(40),
		progressbar.OptionEnableColorCodes(true),
		progressbar.OptionSetWriter(os.Stderr),
		progressbar.OptionSetPredictTime(false),
		progressbar.OptionShowDescriptionAtLineEnd(),
		progressbar.OptionSetElapsedTime(false),
		progressbar.OptionClearOnFinish(),
		progressbar.OptionSetTheme(progressbar.Theme{
			Saucer:        "=",
			SaucerHead:    ">",
			SaucerPadding: " ",
			BarStart:      "Encoding [",
			BarEnd:        "]",
		}),
	)
}

func (r *TerminalReporter) EncodingProgress(progress ProgressSnapshot) {
	r.mu.Lock()
	defer r.mu.Unlock()

	if r.progress == nil {
		return
	}

	clamped := progress.Percent
	if clamped > 100 {
		clamped = 100
	}
	if clamped < 0 {
		clamped = 0
	}

	if clamped >= r.maxPercent {
		r.maxPercent = clamped
		_ = r.progress.Set64(int64(clamped))
	}

	desc := fmt.Sprintf("speed %.1fx, fps %.1f, eta %s",
		progress.Speed, progress.FPS, util.FormatDurationFromSecs(int64(progress.ETA.Seconds())))
	r.progress.Describe(desc)
}

func (r *TerminalReporter) ValidationComplete(summary ValidationSummary) {
	r.finishProgress()

	fmt.Println()
	_, _ = r.cyan.Println("VALIDATION")

	if summary.Passed {
		fmt.Printf("  %s\n", r.green.Add(color.Bold).Sprint("All checks passed"))
	} else {
		fmt.Printf("  %s\n", r.red.Sprint("Validation failed"))
	}

	// Find the longest step name for alignment
	maxLen := 0
	for _, step := range summary.Steps {
		if len(step.Name) > maxLen {
			maxLen = len(step.Name)
		}
	}

	for _, step := range summary.Steps {
		var status string
		if step.Passed {
			status = r.green.Sprint("✓")
		} else {
			status = r.red.Sprint("✗")
		}
		// Pad the name for alignment
		paddedName := fmt.Sprintf("%-*s", maxLen, step.Name)
		fmt.Printf("  - %s: %s (%s)\n", paddedName, status, step.Details)
	}
}

func (r *TerminalReporter) EncodingComplete(summary EncodingOutcome) {
	reduction := util.CalculateSizeReduction(summary.OriginalSize, summary.EncodedSize)

	fmt.Println()
	_, _ = r.cyan.Println("RESULTS")
	fmt.Printf("  %s %s\n", r.bold.Sprint("Output:"), r.bold.Sprint(summary.OutputFile))
	fmt.Printf("  %s %s -> %s\n",
		r.bold.Sprint("Size:"),
		util.FormatBytesReadable(summary.OriginalSize),
		util.FormatBytesReadable(summary.EncodedSize))
	fmt.Printf("  %s %s\n", r.bold.Sprint("Reduction:"), r.bold.Sprintf("%.1f%%", reduction))
	r.printLabel(8, "Video:", summary.VideoStream)
	r.printLabel(8, "Audio:", summary.AudioStream)
	fmt.Printf("  %s %s (avg speed %.1fx)\n",
		r.bold.Sprint("Time:"),
		util.FormatDurationFromSecs(int64(summary.TotalTime.Seconds())),
		summary.AverageSpeed)
	fmt.Printf("  %s %s\n", r.bold.Sprint("Saved to"), r.green.Sprint(summary.OutputPath))
}

func (r *TerminalReporter) Warning(message string) {
	fmt.Println()
	_, _ = r.yellow.Printf("WARN: %s\n", message)
}

func (r *TerminalReporter) Error(err ReporterError) {
	_, _ = fmt.Fprintln(os.Stderr)
	_, _ = r.red.Fprintf(os.Stderr, "ERROR %s\n", err.Title)
	_, _ = fmt.Fprintf(os.Stderr, "  %s\n", err.Message)
	if err.Context != "" {
		_, _ = fmt.Fprintf(os.Stderr, "  Context: %s\n", err.Context)
	}
	if err.Suggestion != "" {
		_, _ = fmt.Fprintf(os.Stderr, "  Suggestion: %s\n", err.Suggestion)
	}
}

func (r *TerminalReporter) OperationComplete(message string) {
	fmt.Println()
	fmt.Printf("%s %s\n", r.green.Add(color.Bold).Sprint("✓"), r.bold.Sprint(message))
}

func (r *TerminalReporter) BatchStarted(info BatchStartInfo) {
	fmt.Println()
	_, _ = r.cyan.Println("BATCH")
	fmt.Printf("  Processing %d files -> %s\n", info.TotalFiles, r.bold.Sprint(info.OutputDir))
	for i, name := range info.FileList {
		fmt.Printf("  %d. %s\n", i+1, name)
	}
}

func (r *TerminalReporter) FileProgress(context FileProgressContext) {
	fmt.Printf("\nFile %s of %d\n",
		r.bold.Sprint(context.CurrentFile),
		context.TotalFiles)
}

func (r *TerminalReporter) BatchComplete(summary BatchSummary) {
	reduction := util.CalculateSizeReduction(summary.TotalOriginalSize, summary.TotalEncodedSize)

	fmt.Println()
	_, _ = r.cyan.Println("BATCH SUMMARY")
	fmt.Printf("  %s\n", r.bold.Sprintf("%d of %d succeeded", summary.SuccessfulCount, summary.TotalFiles))
	fmt.Printf("  Validation: %s passed, %s failed\n",
		r.green.Sprint(summary.ValidationPassedCount),
		r.red.Sprint(summary.ValidationFailedCount))
	fmt.Printf("  Size: %d -> %d bytes (%.1f%% reduction)\n",
		summary.TotalOriginalSize, summary.TotalEncodedSize, reduction)
	fmt.Printf("  Time: %s (avg speed %.1fx)\n",
		util.FormatDurationFromSecs(int64(summary.TotalDuration.Seconds())),
		summary.AverageSpeed)

	for _, result := range summary.FileResults {
		fmt.Printf("  - %s (%.1f%% reduction)\n", result.Filename, result.Reduction)
	}
}
