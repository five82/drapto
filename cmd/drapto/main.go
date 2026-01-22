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
	inputPath      string
	outputDir      string
	logDir         string
	verbose        bool
	qualitySD      uint
	qualityHD      uint
	qualityUHD     uint
	preset         uint
	draptoPreset   string
	disableAutocrop bool
	responsive     bool
	noLog          bool
}

func runEncode(args []string) error {
	fs := flag.NewFlagSet("encode", flag.ExitOnError)
	fs.Usage = func() {
		fmt.Fprintf(os.Stderr, `Encode video files to AV1 format.

Usage:
  %s encode [options]

Required:
  -i, --input <PATH>     Input video file or directory containing video files
  -o, --output <PATH>    Output directory (or filename if input is a single file)

Options:
  -l, --log-dir <PATH>   Log directory (defaults to ~/.local/state/drapto/logs)
  -v, --verbose          Enable verbose output for troubleshooting

Quality Settings:
  --quality-sd <CRF>     CRF quality for SD videos (<1920 width). Default: %d
  --quality-hd <CRF>     CRF quality for HD videos (≥1920 width). Default: %d
  --quality-uhd <CRF>    CRF quality for UHD videos (≥3840 width). Default: %d
  --preset <0-13>        SVT-AV1 encoder preset. Lower=slower/better. Default: %d
  --drapto-preset <NAME> Apply grouped Drapto defaults (grain, clean, quick)

Processing Options:
  --disable-autocrop     Disable automatic black bar crop detection
  --responsive           Reserve CPU threads for improved system responsiveness

Output Options:
  --no-log               Disable Drapto log file creation
`, appName, config.DefaultQualitySD, config.DefaultQualityHD, config.DefaultQualityUHD, config.DefaultSVTAV1Preset)
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

	// Processing options
	fs.BoolVar(&ea.disableAutocrop, "disable-autocrop", false, "Disable automatic crop detection")
	fs.BoolVar(&ea.responsive, "responsive", false, "Reserve CPU threads for responsiveness")

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
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return fmt.Errorf("failed to get home directory: %w", err)
		}
		logDir = filepath.Join(homeDir, ".local", "state", "drapto", "logs")
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
