package ffmpeg

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/five82/drapto/internal/util"
)

// Progress represents encoding progress information.
type Progress struct {
	CurrentFrame uint64
	TotalFrames  uint64
	Percent      float32
	Speed        float32
	FPS          float32
	ETA          time.Duration
	Bitrate      string
	ElapsedSecs  float64
}

// ProgressCallback is called with progress updates during encoding.
type ProgressCallback func(Progress)

// Result contains the result of an FFmpeg encode operation.
type Result struct {
	Success bool
	Error   error
	Stderr  string
}

var timeRegex = regexp.MustCompile(`time=(\d{2}:\d{2}:\d{2}\.?\d*)`)

// RunEncode executes an FFmpeg encode operation with progress reporting.
func RunEncode(ctx context.Context, params *EncodeParams, disableAudio bool, totalFrames uint64, callback ProgressCallback) Result {
	args := BuildCommand(params, disableAudio)

	cmd := exec.CommandContext(ctx, "ffmpeg", args...)

	// Get stderr for progress parsing
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return Result{
			Success: false,
			Error:   fmt.Errorf("failed to get stderr pipe: %w", err),
		}
	}

	// Start the process
	if err := cmd.Start(); err != nil {
		return Result{
			Success: false,
			Error:   fmt.Errorf("failed to start ffmpeg: %w", err),
		}
	}

	// Parse progress from stderr
	var stderrBuilder strings.Builder
	parseProgress(stderr, &stderrBuilder, params.Duration, totalFrames, callback)

	// Wait for completion
	err = cmd.Wait()
	stderrStr := stderrBuilder.String()

	if err != nil {
		// Check for context cancellation
		if ctx.Err() != nil {
			return Result{
				Success: false,
				Error:   fmt.Errorf("encoding cancelled: %w", ctx.Err()),
				Stderr:  stderrStr,
			}
		}
		// Check for specific error types
		if strings.Contains(stderrStr, "No streams found") {
			return Result{
				Success: false,
				Error:   fmt.Errorf("no streams found in input file"),
				Stderr:  stderrStr,
			}
		}
		return Result{
			Success: false,
			Error:   fmt.Errorf("ffmpeg failed: %w", err),
			Stderr:  stderrStr,
		}
	}

	return Result{
		Success: true,
		Stderr:  stderrStr,
	}
}

// parseProgress reads FFmpeg stderr and parses progress updates.
func parseProgress(stderr io.Reader, stderrBuilder *strings.Builder, duration float64, totalFrames uint64, callback ProgressCallback) {
	reader := bufio.NewReader(stderr)
	var lineBuf strings.Builder

	for {
		b, err := reader.ReadByte()
		if err != nil {
			if err != io.EOF {
				fmt.Printf("Error reading stderr: %v\n", err)
			}
			break
		}

		stderrBuilder.WriteByte(b)

		// Progress lines end with \r or \n
		if b == '\r' || b == '\n' {
			line := lineBuf.String()
			lineBuf.Reset()

			if callback != nil && strings.Contains(line, "frame=") {
				progress := parseProgressLine(line, duration, totalFrames)
				if progress != nil {
					callback(*progress)
				}
			}
		} else {
			lineBuf.WriteByte(b)
		}
	}
}

// parseProgressLine extracts progress information from an FFmpeg progress line.
func parseProgressLine(line string, duration float64, totalFrames uint64) *Progress {
	// Extract elapsed time
	var elapsedSecs float64
	if matches := timeRegex.FindStringSubmatch(line); len(matches) >= 2 {
		if secs, ok := util.ParseFFmpegTime(matches[1]); ok {
			elapsedSecs = secs
		}
	}

	// Extract frame, fps, bitrate, speed
	var frame uint64
	var fps, speed float32
	var bitrate string

	// Parse frame
	if idx := strings.Index(line, "frame="); idx >= 0 {
		remaining := line[idx+6:]
		remaining = strings.TrimLeft(remaining, " ")
		if spaceIdx := strings.IndexAny(remaining, " \t"); spaceIdx > 0 {
			if f, err := strconv.ParseUint(remaining[:spaceIdx], 10, 64); err == nil {
				frame = f
			}
		}
	}

	// Parse fps
	if idx := strings.Index(line, "fps="); idx >= 0 {
		remaining := line[idx+4:]
		remaining = strings.TrimLeft(remaining, " ")
		if spaceIdx := strings.IndexAny(remaining, " \t"); spaceIdx > 0 {
			if f, err := strconv.ParseFloat(remaining[:spaceIdx], 32); err == nil {
				fps = float32(f)
			}
		}
	}

	// Parse bitrate
	if idx := strings.Index(line, "bitrate="); idx >= 0 {
		remaining := line[idx+8:]
		remaining = strings.TrimLeft(remaining, " ")
		if spaceIdx := strings.IndexAny(remaining, " \t"); spaceIdx > 0 {
			bitrate = remaining[:spaceIdx]
		}
	}

	// Parse speed
	if idx := strings.Index(line, "speed="); idx >= 0 {
		remaining := line[idx+6:]
		remaining = strings.TrimLeft(remaining, " ")
		remaining = strings.TrimSuffix(remaining, "x")
		if spaceIdx := strings.IndexAny(remaining, " \t\rx\n"); spaceIdx > 0 {
			remaining = remaining[:spaceIdx]
		}
		remaining = strings.TrimSuffix(remaining, "x")
		if s, err := strconv.ParseFloat(remaining, 32); err == nil {
			speed = float32(s)
		}
	}

	// Calculate percent
	var percent float32
	if duration > 0 {
		percent = float32((elapsedSecs / duration) * 100)
		if percent > 100 {
			percent = 100
		}
	}

	// Calculate ETA
	var eta time.Duration
	if speed > 0 && duration > 0 {
		remainingDuration := duration - elapsedSecs
		etaSeconds := remainingDuration / float64(speed)
		eta = time.Duration(etaSeconds) * time.Second
	}

	return &Progress{
		CurrentFrame: frame,
		TotalFrames:  totalFrames,
		Percent:      percent,
		Speed:        speed,
		FPS:          fps,
		ETA:          eta,
		Bitrate:      bitrate,
		ElapsedSecs:  elapsedSecs,
	}
}
