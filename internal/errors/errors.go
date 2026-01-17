// Package errors provides structured error types for drapto operations.
package errors

import (
	"errors"
	"fmt"
	"os/exec"
)

// ErrorKind represents the category of an error.
type ErrorKind int

const (
	// KindIO represents I/O errors.
	KindIO ErrorKind = iota
	// KindPath represents path-related errors.
	KindPath
	// KindCommand represents external command execution errors.
	KindCommand
	// KindFFmpeg represents FFmpeg-specific errors.
	KindFFmpeg
	// KindFFprobeParse represents FFprobe output parsing errors.
	KindFFprobeParse
	// KindJSONParse represents JSON parsing errors.
	KindJSONParse
	// KindVideoInfo represents video information extraction errors.
	KindVideoInfo
	// KindConfig represents configuration validation errors.
	KindConfig
	// KindNoFilesFound represents no suitable video files found.
	KindNoFilesFound
	// KindOperationFailed represents general operation failures.
	KindOperationFailed
	// KindAnalysis represents analysis-related errors.
	KindAnalysis
	// KindNoStreamsFound represents FFmpeg reporting no streams found.
	KindNoStreamsFound
	// KindCancelled represents user-cancelled operations.
	KindCancelled
)

// String returns a string representation of the error kind.
func (k ErrorKind) String() string {
	switch k {
	case KindIO:
		return "I/O error"
	case KindPath:
		return "Path error"
	case KindCommand:
		return "Command error"
	case KindFFmpeg:
		return "FFmpeg error"
	case KindFFprobeParse:
		return "FFprobe parse error"
	case KindJSONParse:
		return "JSON parse error"
	case KindVideoInfo:
		return "Video info error"
	case KindConfig:
		return "Configuration error"
	case KindNoFilesFound:
		return "No files found"
	case KindOperationFailed:
		return "Operation failed"
	case KindAnalysis:
		return "Analysis error"
	case KindNoStreamsFound:
		return "No streams found"
	case KindCancelled:
		return "Operation cancelled"
	default:
		return "Unknown error"
	}
}

// CommandErrorKind represents the type of command error.
type CommandErrorKind int

const (
	// CommandStart means the command failed to start.
	CommandStart CommandErrorKind = iota
	// CommandWait means waiting for the command failed.
	CommandWait
	// CommandFailed means the command returned non-zero exit status.
	CommandFailed
)

// CommandError represents an error from executing an external command.
type CommandError struct {
	Command    string
	Kind       CommandErrorKind
	ExitCode   int
	Stderr     string
	Underlying error
}

func (e *CommandError) Error() string {
	switch e.Kind {
	case CommandStart:
		return fmt.Sprintf("failed to execute %s: %v", e.Command, e.Underlying)
	case CommandWait:
		return fmt.Sprintf("failed to wait for %s: %v", e.Command, e.Underlying)
	case CommandFailed:
		if e.Stderr != "" {
			return fmt.Sprintf("command %s failed with exit code %d: %s", e.Command, e.ExitCode, e.Stderr)
		}
		return fmt.Sprintf("command %s failed with exit code %d", e.Command, e.ExitCode)
	default:
		return fmt.Sprintf("command %s error: %v", e.Command, e.Underlying)
	}
}

func (e *CommandError) Unwrap() error {
	return e.Underlying
}

// CoreError is the main error type for drapto operations.
type CoreError struct {
	Kind       ErrorKind
	Message    string
	Underlying error
}

func (e *CoreError) Error() string {
	if e.Underlying != nil {
		return fmt.Sprintf("%s: %s: %v", e.Kind, e.Message, e.Underlying)
	}
	return fmt.Sprintf("%s: %s", e.Kind, e.Message)
}

func (e *CoreError) Unwrap() error {
	return e.Underlying
}

// Is reports whether target matches this error's kind.
func (e *CoreError) Is(target error) bool {
	t, ok := target.(*CoreError)
	if !ok {
		return false
	}
	return e.Kind == t.Kind
}

// NewIOError creates a new I/O error.
func NewIOError(message string, underlying error) *CoreError {
	return &CoreError{Kind: KindIO, Message: message, Underlying: underlying}
}

// NewPathError creates a new path-related error.
func NewPathError(message string) *CoreError {
	return &CoreError{Kind: KindPath, Message: message}
}

// NewCommandError creates a new command execution error.
func NewCommandError(cmd string, kind CommandErrorKind, underlying error) *CoreError {
	cmdErr := &CommandError{
		Command:    cmd,
		Kind:       kind,
		Underlying: underlying,
	}
	return &CoreError{Kind: KindCommand, Message: cmdErr.Error(), Underlying: cmdErr}
}

// NewCommandStartError creates an error for when a command fails to start.
func NewCommandStartError(cmd string, err error) *CoreError {
	return NewCommandError(cmd, CommandStart, err)
}

// NewCommandWaitError creates an error for when waiting for a command fails.
func NewCommandWaitError(cmd string, err error) *CoreError {
	return NewCommandError(cmd, CommandWait, err)
}

// NewCommandFailedError creates an error for when a command returns non-zero exit status.
func NewCommandFailedError(cmd string, exitCode int, stderr string) *CoreError {
	cmdErr := &CommandError{
		Command:  cmd,
		Kind:     CommandFailed,
		ExitCode: exitCode,
		Stderr:   stderr,
	}
	return &CoreError{Kind: KindCommand, Message: cmdErr.Error(), Underlying: cmdErr}
}

// NewFFmpegError creates a new FFmpeg-specific error.
func NewFFmpegError(message string) *CoreError {
	return &CoreError{Kind: KindFFmpeg, Message: message}
}

// NewFFprobeParseError creates a new FFprobe parsing error.
func NewFFprobeParseError(message string) *CoreError {
	return &CoreError{Kind: KindFFprobeParse, Message: message}
}

// NewJSONParseError creates a new JSON parsing error.
func NewJSONParseError(message string, underlying error) *CoreError {
	return &CoreError{Kind: KindJSONParse, Message: message, Underlying: underlying}
}

// NewVideoInfoError creates a new video information extraction error.
func NewVideoInfoError(message string) *CoreError {
	return &CoreError{Kind: KindVideoInfo, Message: message}
}

// NewConfigError creates a new configuration error.
func NewConfigError(message string) *CoreError {
	return &CoreError{Kind: KindConfig, Message: message}
}

// NewNoFilesFoundError creates an error for when no video files are found.
func NewNoFilesFoundError(dir string) *CoreError {
	return &CoreError{Kind: KindNoFilesFound, Message: fmt.Sprintf("no suitable video files found in %s", dir)}
}

// NewOperationFailedError creates a new general operation failure error.
func NewOperationFailedError(message string, underlying error) *CoreError {
	return &CoreError{Kind: KindOperationFailed, Message: message, Underlying: underlying}
}

// NewAnalysisError creates a new analysis-related error.
func NewAnalysisError(message string) *CoreError {
	return &CoreError{Kind: KindAnalysis, Message: message}
}

// NewNoStreamsFoundError creates an error for when FFmpeg reports no streams.
func NewNoStreamsFoundError(path string) *CoreError {
	return &CoreError{Kind: KindNoStreamsFound, Message: fmt.Sprintf("no streams found in %s", path)}
}

// NewCancelledError creates an error for user-cancelled operations.
func NewCancelledError() *CoreError {
	return &CoreError{Kind: KindCancelled, Message: "operation was cancelled by the user"}
}

// IsKind checks if the error has the specified kind.
func IsKind(err error, kind ErrorKind) bool {
	var coreErr *CoreError
	if errors.As(err, &coreErr) {
		return coreErr.Kind == kind
	}
	return false
}

// IsCancelled checks if the error is a cancellation error.
func IsCancelled(err error) bool {
	return IsKind(err, KindCancelled)
}

// IsNoFilesFound checks if the error is a no-files-found error.
func IsNoFilesFound(err error) bool {
	return IsKind(err, KindNoFilesFound)
}

// WrapExecError wraps an exec.ExitError into a CoreError.
func WrapExecError(cmd string, err error, stderr string) *CoreError {
	if exitErr, ok := err.(*exec.ExitError); ok {
		return NewCommandFailedError(cmd, exitErr.ExitCode(), stderr)
	}
	return NewCommandStartError(cmd, err)
}
