// Package logging provides structured logging infrastructure for drapto.
package logging

import (
	"io"
	"log/slog"
	"os"
	"sync"
)

// Level aliases for slog levels.
const (
	LevelDebug = slog.LevelDebug
	LevelInfo  = slog.LevelInfo
	LevelWarn  = slog.LevelWarn
	LevelError = slog.LevelError
)

// Logger wraps slog.Logger with drapto-specific configuration.
type Logger struct {
	*slog.Logger
}

// Config contains logger configuration options.
type Config struct {
	Level   slog.Level
	Output  io.Writer
	Enabled bool
}

// DefaultConfig returns a default logger configuration.
func DefaultConfig() Config {
	return Config{
		Level:   LevelInfo,
		Output:  os.Stderr,
		Enabled: true,
	}
}

// New creates a new logger with the given configuration.
func New(cfg Config) *Logger {
	if !cfg.Enabled {
		// Return a no-op logger that discards all output
		return &Logger{
			Logger: slog.New(slog.NewTextHandler(io.Discard, nil)),
		}
	}

	output := cfg.Output
	if output == nil {
		output = os.Stderr
	}

	handler := slog.NewTextHandler(output, &slog.HandlerOptions{
		Level: cfg.Level,
	})

	return &Logger{
		Logger: slog.New(handler),
	}
}

// WithPrefix returns a new logger with the given prefix as a group.
func (l *Logger) WithPrefix(prefix string) *Logger {
	return &Logger{
		Logger: l.WithGroup(prefix),
	}
}

// Global logger instance.
var (
	globalLogger     *Logger
	globalLoggerOnce sync.Once
)

// Global returns the global logger instance.
func Global() *Logger {
	globalLoggerOnce.Do(func() {
		globalLogger = New(DefaultConfig())
	})
	return globalLogger
}

// SetGlobal sets the global logger instance.
func SetGlobal(logger *Logger) {
	globalLogger = logger
}

// Init initializes the global logger with the given level and output.
func Init(level slog.Level, w io.Writer) {
	SetGlobal(New(Config{
		Level:   level,
		Output:  w,
		Enabled: true,
	}))
}

// Package-level convenience functions that delegate to the global logger.

// Debug logs a debug message to the global logger.
func Debug(msg string, args ...any) {
	Global().Debug(msg, args...)
}

// Info logs an informational message to the global logger.
func Info(msg string, args ...any) {
	Global().Info(msg, args...)
}

// Warn logs a warning message to the global logger.
func Warn(msg string, args ...any) {
	Global().Warn(msg, args...)
}

// Error logs an error message to the global logger.
func Error(msg string, args ...any) {
	Global().Error(msg, args...)
}
