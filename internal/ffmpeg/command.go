package ffmpeg

import (
	"fmt"
	"strings"

	"github.com/five82/drapto/internal/ffprobe"
)

// Audio bitrate constants (kbps)
const (
	audioBitrateMono       = 64  // kbps for 1 channel
	audioBitrateStereo     = 128 // kbps for 2 channels
	audioBitrateSurround51 = 256 // kbps for 5.1 surround
	audioBitrateSurround71 = 384 // kbps for 7.1 surround
	audioBitratePerChan    = 48  // kbps per channel (fallback for non-standard configs)
)

// SVT-AV1 default parameters
const (
	svtav1DefaultKeyint = "10s"
	svtav1DefaultSCD    = "1" // scene change detection enabled
	svtav1DefaultSCM    = "0" // scene change mode
)

// EncodeParams contains all parameters for an FFmpeg encode operation.
type EncodeParams struct {
	InputPath             string
	OutputPath            string
	Quality               uint32
	Preset                uint8
	Tune                  uint8
	ACBias                float32
	EnableVarianceBoost   bool
	VarianceBoostStrength uint8
	VarianceOctile        uint8
	VideoDenoiseFilter string // Optional denoise filter
	FilmGrain          *uint8 // Optional film grain synthesis strength
	FilmGrainDenoise   *bool  // Optional film grain denoise toggle
	LowPriority        bool   // Run encoder at low priority (nice -n 19)
	CropFilter         string // Optional crop filter
	AudioChannels         []uint32
	AudioStreams          []ffprobe.AudioStreamInfo
	Duration              float64
	VideoCodec            string
	PixelFormat           string
	MatrixCoefficients    string
	AudioCodec            string
}

// SVTAV1CLIParams builds the -svtav1-params string.
func (p *EncodeParams) SVTAV1CLIParams() string {
	builder := NewSvtAv1ParamsBuilder().
		WithACBias(p.ACBias).
		WithEnableVarianceBoost(p.EnableVarianceBoost)

	if p.EnableVarianceBoost {
		builder = builder.
			WithVarianceBoostStrength(p.VarianceBoostStrength).
			WithVarianceOctile(p.VarianceOctile)
	}

	builder = builder.WithTune(p.Tune).
		AddParam("keyint", svtav1DefaultKeyint).
		AddParam("scd", svtav1DefaultSCD).
		AddParam("scm", svtav1DefaultSCM)

	if p.FilmGrain != nil {
		builder = builder.AddParam("film-grain", fmt.Sprintf("%d", *p.FilmGrain))
		if p.FilmGrainDenoise != nil {
			val := "0"
			if *p.FilmGrainDenoise {
				val = "1"
			}
			builder = builder.AddParam("film-grain-denoise", val)
		}
	}

	return builder.Build()
}

// BuildCommand builds the FFmpeg command arguments.
func BuildCommand(params *EncodeParams, disableAudio bool) []string {
	args := []string{"-hide_banner", "-i", params.InputPath}

	// Build video filter chain
	filterChain := NewVideoFilterChain()
	if params.CropFilter != "" {
		filterChain.AddCrop(params.CropFilter)
	}
	if params.VideoDenoiseFilter != "" {
		filterChain.AddFilter(params.VideoDenoiseFilter)
	}

	if !filterChain.IsEmpty() {
		args = append(args, "-vf", filterChain.Build())
	}

	// Video encoding configuration
	args = append(args, "-c:v", params.VideoCodec)
	args = append(args, "-pix_fmt", params.PixelFormat)
	args = append(args, "-crf", fmt.Sprintf("%d", params.Quality))
	args = append(args, "-preset", fmt.Sprintf("%d", params.Preset))

	// SVT-AV1 parameters
	svtParams := params.SVTAV1CLIParams()
	args = append(args, "-svtav1-params", svtParams)

	if !disableAudio {
		// Map video stream
		args = append(args, "-map", "0:v:0")

		// Handle audio streams
		if len(params.AudioStreams) > 0 {
			for outputIdx, stream := range params.AudioStreams {
				// Map this audio stream
				args = append(args, "-map", fmt.Sprintf("0:a:%d", stream.Index))

				// Preserve stream metadata
				args = append(args, fmt.Sprintf("-map_metadata:s:a:%d", outputIdx), fmt.Sprintf("0:s:a:%d", stream.Index))

				// Build disposition value
				disposition := buildDispositionValue(stream.Disposition)
				args = append(args, fmt.Sprintf("-disposition:a:%d", outputIdx), disposition)

				// Set audio codec and bitrate
				args = append(args, fmt.Sprintf("-c:a:%d", outputIdx), params.AudioCodec)
				bitrate := calculateAudioBitrate(stream.Channels)
				args = append(args, fmt.Sprintf("-b:a:%d", outputIdx), fmt.Sprintf("%dk", bitrate))

				// Apply audio format filter
				args = append(args, fmt.Sprintf("-filter:a:%d", outputIdx), "aformat=channel_layouts=7.1|5.1|stereo|mono")
			}
		} else if len(params.AudioChannels) > 0 {
			// Fallback to old behavior if no detailed stream info
			args = append(args, "-c:a", params.AudioCodec)
			for i, channels := range params.AudioChannels {
				bitrate := calculateAudioBitrate(channels)
				args = append(args, fmt.Sprintf("-b:a:%d", i), fmt.Sprintf("%dk", bitrate))
				args = append(args, fmt.Sprintf("-filter:a:%d", i), "aformat=channel_layouts=7.1|5.1|stereo|mono")
			}
			args = append(args, "-map", "0:a")
		}

		args = append(args, "-map_metadata", "0")
		args = append(args, "-map_chapters", "0")
	} else {
		args = append(args, "-map", "0:v:0")
		args = append(args, "-an")
	}

	args = append(args, "-movflags", "+faststart")
	args = append(args, params.OutputPath)

	return args
}

// buildDispositionValue builds a disposition value string from disposition flags.
func buildDispositionValue(d ffprobe.StreamDisposition) string {
	var flags []string

	if d.Default != 0 {
		flags = append(flags, "default")
	}
	if d.Dub != 0 {
		flags = append(flags, "dub")
	}
	if d.Original != 0 {
		flags = append(flags, "original")
	}
	if d.Comment != 0 {
		flags = append(flags, "comment")
	}
	if d.Lyrics != 0 {
		flags = append(flags, "lyrics")
	}
	if d.Karaoke != 0 {
		flags = append(flags, "karaoke")
	}
	if d.Forced != 0 {
		flags = append(flags, "forced")
	}
	if d.HearingImpaired != 0 {
		flags = append(flags, "hearing_impaired")
	}
	if d.VisualImpaired != 0 {
		flags = append(flags, "visual_impaired")
	}
	if d.CleanEffects != 0 {
		flags = append(flags, "clean_effects")
	}
	if d.AttachedPic != 0 {
		flags = append(flags, "attached_pic")
	}
	if d.TimedThumbnails != 0 {
		flags = append(flags, "timed_thumbnails")
	}

	if len(flags) == 0 {
		return "0"
	}
	return strings.Join(flags, "+")
}

// calculateAudioBitrate returns audio bitrate in kbps based on channel count.
func calculateAudioBitrate(channels uint32) uint32 {
	switch channels {
	case 1:
		return audioBitrateMono
	case 2:
		return audioBitrateStereo
	case 6:
		return audioBitrateSurround51
	case 8:
		return audioBitrateSurround71
	default:
		return channels * audioBitratePerChan
	}
}

// CalculateAudioBitrate is exported for use by other packages.
func CalculateAudioBitrate(channels uint32) uint32 {
	return calculateAudioBitrate(channels)
}
