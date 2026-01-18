// Package main provides the CLI entry point for Drapto.
package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"

	"github.com/five82/drapto/internal/config"
	"github.com/five82/drapto/internal/discovery"
	"github.com/five82/drapto/internal/logging"
	"github.com/five82/drapto/internal/processing"
	"github.com/five82/drapto/internal/reporter"
	"github.com/five82/drapto/internal/util"
)

const (
	appName    = "drapto"
	appVersion = "0.2.0"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(1)
	}

	switch os.Args[1] {
	case "encode":
		if err := runEncode(os.Args[2:]); err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}
	case "version", "--version", "-v":
		fmt.Printf("%s version %s\n", appName, appVersion)
	case "help", "--help", "-h":
		printUsage()
	default:
		fmt.Fprintf(os.Stderr, "Unknown command: %s\n\n", os.Args[1])
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Printf(`%s - Video encoding tool

Usage:
  %s <command> [options]

Commands:
  encode    Encode video files to AV1 format
  version   Print version information
  help      Show this help message

Run '%s encode --help' for encode command options.
`, appName, appName, appName)
}

// encodeArgs holds the parsed arguments for the encode command.
type encodeArgs struct {
	inputPath       string
	outputDir       string
	logDir          string
	verbose         bool
	qualitySD       uint
	qualityHD       uint
	qualityUHD      uint
	preset          uint
	draptoPreset    string
	disableAutocrop bool
	responsive      bool
	noLog           bool
	workers         int
	chunkBuffer     int
	// Target quality options
	targetQuality string
	qpRange       string
	metricWorkers int
	metricMode    string
	// Scene detection options
	sceneThreshold float64
	// Sample-based TQ probing options
	sampleDuration    float64
	sampleMinChunk    float64
	disableTQSampling bool
}

func runEncode(args []string) error {
	// Get auto-detected defaults for parallel encoding
	defaultWorkers, defaultBuffer := config.AutoParallelConfig()

	fs := flag.NewFlagSet("encode", flag.ExitOnError)
	fs.Usage = func() {
		fmt.Fprintf(os.Stderr, `Encode video files to AV1 format.

Usage:
  %s encode [options]

Required:
  -i, --input <PATH>     Input video file or directory containing video files
  -o, --output <PATH>    Output directory (or filename if input is a single file)

Options:
  -l, --log-dir <PATH>   Log directory (defaults to OUTPUT/logs)
  -v, --verbose          Enable verbose output for troubleshooting

Quality Settings:
  --quality-sd <CRF>     CRF quality for SD videos (<1920 width). Default: %d
  --quality-hd <CRF>     CRF quality for HD videos (≥1920 width). Default: %d
  --quality-uhd <CRF>    CRF quality for UHD videos (≥3840 width). Default: %d
  --preset <0-13>        SVT-AV1 encoder preset. Lower=slower/better. Default: %d
  --drapto-preset <NAME> Apply grouped Drapto defaults (grain, clean, quick)

Target Quality Options (per-chunk SSIMULACRA2 targeting):
  -t, --target <RANGE>   Target SSIMULACRA2 quality range (e.g., "70-75")
  --qp <RANGE>           CRF search range. Default: 8-48
  --metric-workers <N>   Number of GPU metric workers. Default: 1
  --metric-mode <MODE>   Metric aggregation mode ("mean" or "pN"). Default: mean

Processing Options:
  --disable-autocrop     Disable automatic black bar crop detection
  --responsive           Reserve CPU threads for improved system responsiveness
  --workers <N>          Number of parallel encoder workers. Default: %d (auto)
  --buffer <N>           Extra chunks to buffer in memory. Default: %d (auto)
  --scene-threshold <N>  Scene detection threshold (0.0-1.0, higher = fewer scenes). Default: %.1f
  --sample-duration <N>  Seconds to sample for TQ probing. Default: %.1f
  --sample-min-chunk <N> Minimum chunk duration (seconds) to use sampling. Default: %.1f
  --no-tq-sampling       Disable sample-based TQ probing (use full chunks)

Output Options:
  --no-log               Disable Drapto log file creation
`, appName, config.DefaultQualitySD, config.DefaultQualityHD, config.DefaultQualityUHD, config.DefaultSVTAV1Preset, defaultWorkers, defaultBuffer, config.DefaultSceneThreshold, config.DefaultSampleDuration, config.DefaultSampleMinChunk)
	}

	var ea encodeArgs

	// Required arguments
	fs.StringVar(&ea.inputPath, "i", "", "Input video file or directory")
	fs.StringVar(&ea.inputPath, "input", "", "Input video file or directory")
	fs.StringVar(&ea.outputDir, "o", "", "Output directory")
	fs.StringVar(&ea.outputDir, "output", "", "Output directory")

	// Optional arguments
	fs.StringVar(&ea.logDir, "l", "", "Log directory")
	fs.StringVar(&ea.logDir, "log-dir", "", "Log directory")
	fs.BoolVar(&ea.verbose, "v", false, "Enable verbose output")
	fs.BoolVar(&ea.verbose, "verbose", false, "Enable verbose output")

	// Quality settings
	fs.UintVar(&ea.qualitySD, "quality-sd", 0, "CRF quality for SD videos")
	fs.UintVar(&ea.qualityHD, "quality-hd", 0, "CRF quality for HD videos")
	fs.UintVar(&ea.qualityUHD, "quality-uhd", 0, "CRF quality for UHD videos")
	fs.UintVar(&ea.preset, "preset", 0, "SVT-AV1 encoder preset (0-13)")
	fs.StringVar(&ea.draptoPreset, "drapto-preset", "", "Drapto preset (grain, clean, quick)")

	// Target quality options
	fs.StringVar(&ea.targetQuality, "t", "", "Target SSIMULACRA2 quality range")
	fs.StringVar(&ea.targetQuality, "target", "", "Target SSIMULACRA2 quality range")
	fs.StringVar(&ea.qpRange, "qp", "8-48", "CRF search range")
	fs.IntVar(&ea.metricWorkers, "metric-workers", 1, "Number of GPU metric workers")
	fs.StringVar(&ea.metricMode, "metric-mode", "mean", "Metric aggregation mode")

	// Processing options
	fs.BoolVar(&ea.disableAutocrop, "disable-autocrop", false, "Disable automatic crop detection")
	fs.BoolVar(&ea.responsive, "responsive", false, "Reserve CPU threads for responsiveness")
	fs.IntVar(&ea.workers, "workers", defaultWorkers, "Number of parallel encoder workers")
	fs.IntVar(&ea.chunkBuffer, "buffer", defaultBuffer, "Extra chunks to buffer in memory")
	fs.Float64Var(&ea.sceneThreshold, "scene-threshold", config.DefaultSceneThreshold, "Scene detection threshold")
	fs.Float64Var(&ea.sampleDuration, "sample-duration", config.DefaultSampleDuration, "Seconds to sample for TQ probing")
	fs.Float64Var(&ea.sampleMinChunk, "sample-min-chunk", config.DefaultSampleMinChunk, "Minimum chunk duration for sampling")
	fs.BoolVar(&ea.disableTQSampling, "no-tq-sampling", false, "Disable sample-based TQ probing")

	// Output options
	fs.BoolVar(&ea.noLog, "no-log", false, "Disable log file creation")

	if err := fs.Parse(args); err != nil {
		return err
	}

	// Validate required arguments
	if ea.inputPath == "" {
		return fmt.Errorf("input path is required (-i/--input)")
	}
	if ea.outputDir == "" {
		return fmt.Errorf("output directory is required (-o/--output)")
	}

	return executeEncode(ea)
}

func executeEncode(ea encodeArgs) error {
	// Resolve input path
	inputPath, err := filepath.Abs(ea.inputPath)
	if err != nil {
		return fmt.Errorf("invalid input path: %w", err)
	}

	// Check if input exists
	inputInfo, err := os.Stat(inputPath)
	if err != nil {
		return fmt.Errorf("input path does not exist: %s", inputPath)
	}

	// Resolve output path
	outputDir, targetFilename, err := resolveOutputPath(inputPath, ea.outputDir, inputInfo.IsDir())
	if err != nil {
		return err
	}

	// Ensure output directory exists
	if err := util.EnsureDirectory(outputDir); err != nil {
		return fmt.Errorf("failed to create output directory: %w", err)
	}

	// Resolve log directory
	logDir := ea.logDir
	if logDir == "" {
		logDir = filepath.Join(outputDir, "logs")
	}

	// Setup file logging
	logger, err := logging.Setup(logDir, ea.verbose, ea.noLog)
	if err != nil {
		return fmt.Errorf("failed to setup logging: %w", err)
	}
	if logger != nil {
		defer func() { _ = logger.Close() }()
	}

	// Discover files to process
	var filesToProcess []string
	if inputInfo.IsDir() {
		filesToProcess, err = discovery.FindVideoFiles(inputPath)
		if err != nil {
			return fmt.Errorf("failed to discover video files: %w", err)
		}
		if len(filesToProcess) == 0 {
			return fmt.Errorf("no video files found in %s", inputPath)
		}
		if logger != nil {
			logger.Info("Discovered %d video files in %s", len(filesToProcess), inputPath)
			for i, f := range filesToProcess {
				logger.Debug("  %d. %s", i+1, f)
			}
		}
	} else {
		filesToProcess = []string{inputPath}
		if logger != nil {
			logger.Info("Processing single file: %s", inputPath)
		}
	}

	// Build configuration
	cfg := config.NewConfig(inputPath, outputDir, logDir)

	// Apply drapto preset first (if specified)
	if ea.draptoPreset != "" {
		preset, err := config.ParsePreset(ea.draptoPreset)
		if err != nil {
			return err
		}
		cfg.ApplyPreset(preset)
	}

	// Override with explicit CLI arguments
	if ea.qualitySD != 0 {
		cfg.QualitySD = uint8(ea.qualitySD)
	}
	if ea.qualityHD != 0 {
		cfg.QualityHD = uint8(ea.qualityHD)
	}
	if ea.qualityUHD != 0 {
		cfg.QualityUHD = uint8(ea.qualityUHD)
	}
	if ea.preset != 0 {
		cfg.SVTAV1Preset = uint8(ea.preset)
	}
	if ea.disableAutocrop {
		cfg.CropMode = "none"
	}
	cfg.ResponsiveEncoding = ea.responsive
	cfg.Workers = ea.workers
	cfg.ChunkBuffer = ea.chunkBuffer

	// Target quality options
	cfg.TargetQuality = ea.targetQuality
	cfg.QPRange = ea.qpRange
	cfg.MetricWorkers = ea.metricWorkers
	cfg.MetricMode = ea.metricMode

	// Scene detection options
	cfg.SceneThreshold = ea.sceneThreshold

	// Sample-based TQ probing options
	cfg.SampleDuration = ea.sampleDuration
	cfg.SampleMinChunk = ea.sampleMinChunk
	cfg.DisableTQSampling = ea.disableTQSampling

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		return fmt.Errorf("invalid configuration: %w", err)
	}

	// Log configuration
	if logger != nil {
		logger.Info("Output directory: %s", outputDir)
		logger.Info("Quality settings: SD=%d, HD=%d, UHD=%d", cfg.QualitySD, cfg.QualityHD, cfg.QualityUHD)
		logger.Info("SVT-AV1 preset: %d", cfg.SVTAV1Preset)
		logger.Info("Crop mode: %s", cfg.CropMode)
		logger.Info("Responsive encoding: %v", cfg.ResponsiveEncoding)
		logger.Info("Parallel encoding: workers=%d, buffer=%d", cfg.Workers, cfg.ChunkBuffer)
		if cfg.DraptoPreset != nil {
			logger.Info("Drapto preset: %s", *cfg.DraptoPreset)
		}
	}

	// Create reporter
	rep := reporter.NewTerminalReporter()

	// Setup context with signal handling
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
	}()

	// Run encoding
	_, err = processing.ProcessVideos(ctx, cfg, filesToProcess, targetFilename, rep)
	return err
}

// resolveOutputPath determines the output directory and optional target filename.
// If input is a file and output has a video extension, treat output as target filename.
func resolveOutputPath(_, outputPath string, isInputDir bool) (outputDir, targetFilename string, err error) {
	outputPath, err = filepath.Abs(outputPath)
	if err != nil {
		return "", "", fmt.Errorf("invalid output path: %w", err)
	}

	// If input is a directory, output must be a directory
	if isInputDir {
		return outputPath, "", nil
	}

	// Check if output path looks like a file (has video extension)
	ext := filepath.Ext(outputPath)
	videoExtensions := map[string]bool{
		".mkv": true, ".mp4": true, ".webm": true,
		".avi": true, ".mov": true, ".m4v": true,
	}

	if videoExtensions[ext] {
		// Output is a target filename
		return filepath.Dir(outputPath), filepath.Base(outputPath), nil
	}

	// Output is a directory
	return outputPath, "", nil
}
