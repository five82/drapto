package reporter

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sync"
	"time"

	"github.com/five82/drapto/internal/util"
)

// JSONReporter outputs NDJSON events compatible with Spindle.
type JSONReporter struct {
	writer             io.Writer
	mu                 sync.Mutex
	lastProgressBucket int
	lastProgressTime   time.Time
}

// NewJSONReporter creates a new JSON reporter that writes to stdout.
func NewJSONReporter() *JSONReporter {
	return &JSONReporter{
		writer:             os.Stdout,
		lastProgressBucket: -1,
	}
}

// NewJSONReporterWithWriter creates a JSON reporter with a custom writer.
func NewJSONReporterWithWriter(w io.Writer) *JSONReporter {
	return &JSONReporter{
		writer:             w,
		lastProgressBucket: -1,
	}
}

func (r *JSONReporter) timestamp() int64 {
	return time.Now().Unix()
}

func (r *JSONReporter) write(v interface{}) {
	r.mu.Lock()
	defer r.mu.Unlock()

	data, err := json.Marshal(v)
	if err != nil {
		return
	}
	_, _ = fmt.Fprintln(r.writer, string(data))
}

func (r *JSONReporter) Hardware(summary HardwareSummary) {
	r.write(map[string]interface{}{
		"type":      "hardware",
		"hostname":  summary.Hostname,
		"timestamp": r.timestamp(),
	})
}

func (r *JSONReporter) Initialization(summary InitializationSummary) {
	r.write(map[string]interface{}{
		"type":              "initialization",
		"input_file":        summary.InputFile,
		"output_file":       summary.OutputFile,
		"duration":          summary.Duration,
		"resolution":        summary.Resolution,
		"category":          summary.Category,
		"dynamic_range":     summary.DynamicRange,
		"audio_description": summary.AudioDescription,
		"timestamp":         r.timestamp(),
	})
}

func (r *JSONReporter) StageProgress(update StageProgress) {
	event := map[string]interface{}{
		"type":      "stage_progress",
		"stage":     update.Stage,
		"percent":   update.Percent,
		"message":   update.Message,
		"timestamp": r.timestamp(),
	}
	if update.ETA != nil {
		event["eta_seconds"] = int64(update.ETA.Seconds())
	}
	r.write(event)
}

func (r *JSONReporter) CropResult(summary CropSummary) {
	r.write(map[string]interface{}{
		"type":      "crop_result",
		"message":   summary.Message,
		"crop":      summary.Crop,
		"required":  summary.Required,
		"disabled":  summary.Disabled,
		"timestamp": r.timestamp(),
	})
}

func (r *JSONReporter) EncodingConfig(summary EncodingConfigSummary) {
	presetSettings := make([]map[string]string, len(summary.DraptoPresetSettings))
	for i, kv := range summary.DraptoPresetSettings {
		presetSettings[i] = map[string]string{"key": kv[0], "value": kv[1]}
	}

	r.write(map[string]interface{}{
		"type":                   "encoding_config",
		"encoder":                summary.Encoder,
		"preset":                 summary.Preset,
		"tune":                   summary.Tune,
		"quality":                summary.Quality,
		"pixel_format":           summary.PixelFormat,
		"matrix_coefficients":    summary.MatrixCoefficients,
		"audio_codec":            summary.AudioCodec,
		"audio_description":      summary.AudioDescription,
		"drapto_preset":          summary.DraptoPreset,
		"drapto_preset_settings": presetSettings,
		"svtav1_params":          summary.SVTAV1Params,
		"timestamp":              r.timestamp(),
	})
}

func (r *JSONReporter) EncodingStarted(totalFrames uint64) {
	r.mu.Lock()
	r.lastProgressBucket = -1
	r.lastProgressTime = time.Time{}
	r.mu.Unlock()

	r.write(map[string]interface{}{
		"type":         "encoding_started",
		"total_frames": totalFrames,
		"timestamp":    r.timestamp(),
	})
}

func (r *JSONReporter) EncodingProgress(progress ProgressSnapshot) {
	const progressBucketSize = 1
	const minInterval = 5 * time.Second

	bucket := int(progress.Percent) / progressBucketSize
	now := time.Now()

	r.mu.Lock()
	intervalElapsed := r.lastProgressTime.IsZero() || now.Sub(r.lastProgressTime) >= minInterval
	shouldEmit := bucket > r.lastProgressBucket || intervalElapsed || progress.Percent >= 99.0

	if !shouldEmit {
		r.mu.Unlock()
		return
	}

	if bucket > r.lastProgressBucket {
		r.lastProgressBucket = bucket
	}
	r.lastProgressTime = now
	r.mu.Unlock()

	r.write(map[string]interface{}{
		"type":          "encoding_progress",
		"stage":         "encoding",
		"current_frame": progress.CurrentFrame,
		"total_frames":  progress.TotalFrames,
		"percent":       progress.Percent,
		"speed":         progress.Speed,
		"fps":           progress.FPS,
		"eta_seconds":   int64(progress.ETA.Seconds()),
		"bitrate":       progress.Bitrate,
		"timestamp":     r.timestamp(),
	})
}

func (r *JSONReporter) ValidationComplete(summary ValidationSummary) {
	steps := make([]map[string]interface{}, len(summary.Steps))
	for i, step := range summary.Steps {
		steps[i] = map[string]interface{}{
			"step":    step.Name,
			"passed":  step.Passed,
			"details": step.Details,
		}
	}

	r.write(map[string]interface{}{
		"type":              "validation_complete",
		"validation_passed": summary.Passed,
		"validation_steps":  steps,
		"timestamp":         r.timestamp(),
	})
}

func (r *JSONReporter) EncodingComplete(summary EncodingOutcome) {
	reduction := util.CalculateSizeReduction(summary.OriginalSize, summary.EncodedSize)

	r.write(map[string]interface{}{
		"type":                   "encoding_complete",
		"input_file":             summary.InputFile,
		"output_file":            summary.OutputFile,
		"original_size":          summary.OriginalSize,
		"encoded_size":           summary.EncodedSize,
		"video_stream":           summary.VideoStream,
		"audio_stream":           summary.AudioStream,
		"average_speed":          summary.AverageSpeed,
		"output_path":            summary.OutputPath,
		"duration_seconds":       int64(summary.TotalTime.Seconds()),
		"size_reduction_percent": reduction,
		"timestamp":              r.timestamp(),
	})
}

func (r *JSONReporter) Warning(message string) {
	r.write(map[string]interface{}{
		"type":      "warning",
		"message":   message,
		"timestamp": r.timestamp(),
	})
}

func (r *JSONReporter) Error(err ReporterError) {
	r.write(map[string]interface{}{
		"type":       "error",
		"title":      err.Title,
		"message":    err.Message,
		"context":    err.Context,
		"suggestion": err.Suggestion,
		"timestamp":  r.timestamp(),
	})
}

func (r *JSONReporter) OperationComplete(message string) {
	r.write(map[string]interface{}{
		"type":      "operation_complete",
		"message":   message,
		"timestamp": r.timestamp(),
	})
}

func (r *JSONReporter) BatchStarted(info BatchStartInfo) {
	r.write(map[string]interface{}{
		"type":        "batch_started",
		"total_files": info.TotalFiles,
		"file_list":   info.FileList,
		"output_dir":  info.OutputDir,
		"timestamp":   r.timestamp(),
	})
}

func (r *JSONReporter) FileProgress(context FileProgressContext) {
	r.write(map[string]interface{}{
		"type":         "file_progress",
		"current_file": context.CurrentFile,
		"total_files":  context.TotalFiles,
		"timestamp":    r.timestamp(),
	})
}

func (r *JSONReporter) BatchComplete(summary BatchSummary) {
	reduction := util.CalculateSizeReduction(summary.TotalOriginalSize, summary.TotalEncodedSize)

	r.write(map[string]interface{}{
		"type":                         "batch_complete",
		"successful_count":             summary.SuccessfulCount,
		"total_files":                  summary.TotalFiles,
		"total_original_size":          summary.TotalOriginalSize,
		"total_encoded_size":           summary.TotalEncodedSize,
		"total_duration_seconds":       int64(summary.TotalDuration.Seconds()),
		"total_size_reduction_percent": reduction,
		"timestamp":                    r.timestamp(),
	})
}
