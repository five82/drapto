package util

import (
	"bufio"
	"os"
	"runtime"
	"strconv"
	"strings"
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

// AvailableMemoryBytes returns the available memory in bytes.
// On Linux, this reads MemAvailable from /proc/meminfo.
// Returns 0 if memory cannot be determined.
func AvailableMemoryBytes() uint64 {
	f, err := os.Open("/proc/meminfo")
	if err != nil {
		return 0
	}
	defer func() { _ = f.Close() }()

	scanner := bufio.NewScanner(f)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "MemAvailable:") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				kb, err := strconv.ParseUint(fields[1], 10, 64)
				if err == nil {
					return kb * 1024 // Convert KB to bytes
				}
			}
		}
	}
	return 0
}

// MaxPermitsForMemory calculates the maximum safe number of in-flight chunks
// based on available memory and estimated chunk size.
// chunkMemBytes is the estimated memory per in-flight chunk (YUV data).
// memFraction is the fraction of available memory to use (e.g., 0.7 for 70%).
// Returns at least 1.
func MaxPermitsForMemory(chunkMemBytes uint64, memFraction float64) int {
	available := AvailableMemoryBytes()
	if available == 0 {
		return 1 // Can't determine memory, be conservative
	}

	usable := uint64(float64(available) * memFraction)
	if usable < chunkMemBytes {
		return 1
	}

	permits := int(usable / chunkMemBytes)
	return max(permits, 1)
}
