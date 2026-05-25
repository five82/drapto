package ffmpeg

import (
	"fmt"
	"strconv"

	"github.com/five82/drapto/internal/ffprobe"
)

// BuildAudioCommand builds an FFmpeg command that encodes one source audio stream to Opus.
func BuildAudioCommand(params *EncodeParams, stream ffprobe.AudioStreamInfo, outputPath string) []string {
	return []string{
		"-hide_banner",
		"-i", params.InputPath,
		"-vn", "-sn", "-dn",
		"-map", fmt.Sprintf("0:a:%d", stream.Index),
		"-map_metadata:s:a:0", fmt.Sprintf("0:s:a:%d", stream.Index),
		"-disposition:a:0", buildDispositionValue(stream.Disposition),
		"-c:a", params.AudioCodec,
		"-b:a", fmt.Sprintf("%dk", calculateAudioBitrate(stream.Channels)),
		"-filter:a", "aformat=channel_layouts=7.1|5.1|stereo|mono",
		outputPath,
	}
}

// BuildMuxCommand builds an FFmpeg command that combines encoded video with independently encoded audio.
func BuildMuxCommand(params *EncodeParams, videoPath string, audioPaths []string) []string {
	args := []string{"-hide_banner", "-i", videoPath}
	for _, path := range audioPaths {
		args = append(args, "-i", path)
	}
	metadataInput := len(audioPaths) + 1
	args = append(args, "-i", params.InputPath)

	args = append(args, "-map", "0:v:0")
	for i := range audioPaths {
		args = append(args, "-map", fmt.Sprintf("%d:a:0", i+1))
	}
	args = append(args, "-c", "copy")
	args = append(args, "-map_metadata", strconv.Itoa(metadataInput), "-map_chapters", strconv.Itoa(metadataInput))
	for i, stream := range params.AudioStreams {
		if i >= len(audioPaths) {
			break
		}
		args = append(args, fmt.Sprintf("-disposition:a:%d", i), buildDispositionValue(stream.Disposition))
	}
	args = append(args, "-movflags", "+faststart", params.OutputPath)
	return args
}
