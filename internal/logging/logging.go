// Package logging provides file logging for drapto CLI.
package logging

import (
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"time"
)

// Level represents the logging level.
type Level int

const (
	// LevelInfo is the default logging level.
	LevelInfo Level = iota
	// LevelDebug enables verbose debug logging.
	LevelDebug
)

// Logger wraps the standard logger with level filtering and file output.
type Logger struct {
	level    Level
	logger   *log.Logger
	file     *os.File
	filePath string
}

// Setup creates a new logger that writes to a timestamped log file.
// Returns nil if logging is disabled (noLog=true).
func Setup(logDir string, verbose, noLog bool) (*Logger, error) {
	if noLog {
		return nil, nil
	}

	// Create log directory
	if err := os.MkdirAll(logDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create log directory %s: %w", logDir, err)
	}

	// Generate timestamped filename
	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("drapto_encode_run_%s.log", timestamp)
	filePath := filepath.Join(logDir, filename)

	// Open log file
	file, err := os.OpenFile(filePath, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return nil, fmt.Errorf("failed to create log file %s: %w", filePath, err)
	}

	level := LevelInfo
	if verbose {
		level = LevelDebug
	}

	logger := log.New(file, "", log.LstdFlags)

	l := &Logger{
		level:    level,
		logger:   logger,
		file:     file,
		filePath: filePath,
	}

	// Log startup
	l.Info("Drapto encoder starting")
	if verbose {
		l.Info("Debug level logging enabled")
	}
	l.Info("Log file: %s", filePath)

	return l, nil
}

// Close closes the log file.
func (l *Logger) Close() error {
	if l == nil || l.file == nil {
		return nil
	}
	return l.file.Close()
}

// FilePath returns the path to the log file.
func (l *Logger) FilePath() string {
	if l == nil {
		return ""
	}
	return l.filePath
}

// Info logs an info-level message.
func (l *Logger) Info(format string, args ...any) {
	if l == nil {
		return
	}
	l.logger.Printf("[INFO] "+format, args...)
}

// Debug logs a debug-level message (only if verbose mode is enabled).
func (l *Logger) Debug(format string, args ...any) {
	if l == nil || l.level < LevelDebug {
		return
	}
	l.logger.Printf("[DEBUG] "+format, args...)
}

// Warn logs a warning message.
func (l *Logger) Warn(format string, args ...any) {
	if l == nil {
		return
	}
	l.logger.Printf("[WARN] "+format, args...)
}

// Error logs an error message.
func (l *Logger) Error(format string, args ...any) {
	if l == nil {
		return
	}
	l.logger.Printf("[ERROR] "+format, args...)
}

// Writer returns an io.Writer that writes to the log file.
// Useful for redirecting other loggers or capturing output.
func (l *Logger) Writer() io.Writer {
	if l == nil || l.file == nil {
		return io.Discard
	}
	return l.file
}
