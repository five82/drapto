package ffmpeg

// EncodeParams contains parameters for display purposes.
// Only used for showing encoding configuration to the user.
type EncodeParams struct {
	Quality            uint32
	Preset             uint8
	Tune               uint8
	CropFilter         string // Optional crop filter
	PixelFormat        string
	MatrixCoefficients string
}

// CalculateAudioBitrate returns audio bitrate in kbps based on channel count.
func CalculateAudioBitrate(channels uint32) uint32 {
	switch channels {
	case 1:
		return 64 // Mono
	case 2:
		return 128 // Stereo
	case 6:
		return 256 // 5.1 surround
	case 8:
		return 384 // 7.1 surround
	default:
		return channels * 48 // ~48 kbps per channel for non-standard configs
	}
}
