package util

import (
	"os"
	"runtime"
)

// SystemInfo contains information about the host system.
type SystemInfo struct {
	Hostname string
	NumCPU   int
	OS       string
	Arch     string
}

// GetSystemInfo collects system information.
func GetSystemInfo() SystemInfo {
	hostname, _ := os.Hostname()
	return SystemInfo{
		Hostname: hostname,
		NumCPU:   runtime.NumCPU(),
		OS:       runtime.GOOS,
		Arch:     runtime.GOARCH,
	}
}
