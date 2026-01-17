package errors

import (
	"errors"
	"testing"
)

func TestErrorKindString(t *testing.T) {
	tests := []struct {
		kind     ErrorKind
		expected string
	}{
		{KindIO, "I/O error"},
		{KindPath, "Path error"},
		{KindCommand, "Command error"},
		{KindFFmpeg, "FFmpeg error"},
		{KindFFprobeParse, "FFprobe parse error"},
		{KindJSONParse, "JSON parse error"},
		{KindVideoInfo, "Video info error"},
		{KindConfig, "Configuration error"},
		{KindNoFilesFound, "No files found"},
		{KindOperationFailed, "Operation failed"},
		{KindAnalysis, "Analysis error"},
		{KindNoStreamsFound, "No streams found"},
		{KindCancelled, "Operation cancelled"},
	}

	for _, tt := range tests {
		t.Run(tt.expected, func(t *testing.T) {
			if got := tt.kind.String(); got != tt.expected {
				t.Errorf("ErrorKind.String() = %v, want %v", got, tt.expected)
			}
		})
	}
}

func TestCoreErrorError(t *testing.T) {
	// Test error with underlying error
	underlying := errors.New("underlying error")
	err := &CoreError{
		Kind:       KindIO,
		Message:    "test message",
		Underlying: underlying,
	}

	got := err.Error()
	expected := "I/O error: test message: underlying error"
	if got != expected {
		t.Errorf("CoreError.Error() = %v, want %v", got, expected)
	}

	// Test error without underlying error
	err2 := &CoreError{
		Kind:    KindConfig,
		Message: "config issue",
	}

	got2 := err2.Error()
	expected2 := "Configuration error: config issue"
	if got2 != expected2 {
		t.Errorf("CoreError.Error() = %v, want %v", got2, expected2)
	}
}

func TestCoreErrorUnwrap(t *testing.T) {
	underlying := errors.New("underlying error")
	err := &CoreError{
		Kind:       KindIO,
		Message:    "test",
		Underlying: underlying,
	}

	if err.Unwrap() != underlying {
		t.Error("Unwrap() should return underlying error")
	}
}

func TestCoreErrorIs(t *testing.T) {
	err1 := &CoreError{Kind: KindIO, Message: "test1"}
	err2 := &CoreError{Kind: KindIO, Message: "test2"}
	err3 := &CoreError{Kind: KindConfig, Message: "test3"}

	if !err1.Is(err2) {
		t.Error("Same kind errors should match")
	}

	if err1.Is(err3) {
		t.Error("Different kind errors should not match")
	}
}

func TestCommandError(t *testing.T) {
	// Test CommandStart error
	startErr := &CommandError{
		Command:    "ffmpeg",
		Kind:       CommandStart,
		Underlying: errors.New("not found"),
	}
	if got := startErr.Error(); got != "failed to execute ffmpeg: not found" {
		t.Errorf("CommandStart error = %v", got)
	}

	// Test CommandWait error
	waitErr := &CommandError{
		Command:    "ffprobe",
		Kind:       CommandWait,
		Underlying: errors.New("signal"),
	}
	if got := waitErr.Error(); got != "failed to wait for ffprobe: signal" {
		t.Errorf("CommandWait error = %v", got)
	}

	// Test CommandFailed error
	failedErr := &CommandError{
		Command:  "mediainfo",
		Kind:     CommandFailed,
		ExitCode: 1,
		Stderr:   "file not found",
	}
	expected := "command mediainfo failed with exit code 1: file not found"
	if got := failedErr.Error(); got != expected {
		t.Errorf("CommandFailed error = %v, want %v", got, expected)
	}
}

func TestErrorConstructors(t *testing.T) {
	t.Run("NewIOError", func(t *testing.T) {
		err := NewIOError("disk full", errors.New("no space"))
		if err.Kind != KindIO {
			t.Errorf("Expected KindIO, got %v", err.Kind)
		}
	})

	t.Run("NewPathError", func(t *testing.T) {
		err := NewPathError("invalid path")
		if err.Kind != KindPath {
			t.Errorf("Expected KindPath, got %v", err.Kind)
		}
	})

	t.Run("NewFFmpegError", func(t *testing.T) {
		err := NewFFmpegError("encode failed")
		if err.Kind != KindFFmpeg {
			t.Errorf("Expected KindFFmpeg, got %v", err.Kind)
		}
	})

	t.Run("NewConfigError", func(t *testing.T) {
		err := NewConfigError("invalid preset")
		if err.Kind != KindConfig {
			t.Errorf("Expected KindConfig, got %v", err.Kind)
		}
	})

	t.Run("NewNoFilesFoundError", func(t *testing.T) {
		err := NewNoFilesFoundError("/test/dir")
		if err.Kind != KindNoFilesFound {
			t.Errorf("Expected KindNoFilesFound, got %v", err.Kind)
		}
	})

	t.Run("NewCancelledError", func(t *testing.T) {
		err := NewCancelledError()
		if err.Kind != KindCancelled {
			t.Errorf("Expected KindCancelled, got %v", err.Kind)
		}
	})
}

func TestIsKind(t *testing.T) {
	err := NewConfigError("test")

	if !IsKind(err, KindConfig) {
		t.Error("IsKind should return true for matching kind")
	}

	if IsKind(err, KindIO) {
		t.Error("IsKind should return false for non-matching kind")
	}

	if IsKind(errors.New("plain error"), KindConfig) {
		t.Error("IsKind should return false for non-CoreError")
	}
}

func TestIsCancelled(t *testing.T) {
	cancelledErr := NewCancelledError()
	if !IsCancelled(cancelledErr) {
		t.Error("IsCancelled should return true for cancelled error")
	}

	otherErr := NewConfigError("test")
	if IsCancelled(otherErr) {
		t.Error("IsCancelled should return false for non-cancelled error")
	}
}

func TestIsNoFilesFound(t *testing.T) {
	noFilesErr := NewNoFilesFoundError("/test")
	if !IsNoFilesFound(noFilesErr) {
		t.Error("IsNoFilesFound should return true for no-files-found error")
	}

	otherErr := NewConfigError("test")
	if IsNoFilesFound(otherErr) {
		t.Error("IsNoFilesFound should return false for other errors")
	}
}
