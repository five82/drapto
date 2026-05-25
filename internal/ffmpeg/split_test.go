package ffmpeg

import (
	"reflect"
	"testing"

	"github.com/five82/drapto/internal/ffprobe"
)

func TestBuildAudioCommandEncodesOneStream(t *testing.T) {
	params := &EncodeParams{InputPath: "input.mkv", AudioCodec: "libopus"}
	stream := ffprobe.AudioStreamInfo{
		Index:    2,
		Channels: 8,
		Disposition: ffprobe.StreamDisposition{
			Default: 1,
		},
	}

	got := BuildAudioCommand(params, stream, "audio_02.mka")
	want := []string{
		"-hide_banner",
		"-i", "input.mkv",
		"-vn", "-sn", "-dn",
		"-map", "0:a:2",
		"-map_metadata:s:a:0", "0:s:a:2",
		"-disposition:a:0", "default",
		"-c:a", "libopus",
		"-b:a", "384k",
		"-filter:a", "aformat=channel_layouts=7.1|5.1|stereo|mono",
		"audio_02.mka",
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("BuildAudioCommand() = %#v, want %#v", got, want)
	}
}

func TestBuildMuxCommandCopiesVideoAndIndependentAudio(t *testing.T) {
	params := &EncodeParams{
		InputPath:  "input.mkv",
		OutputPath: "output.mkv",
		AudioStreams: []ffprobe.AudioStreamInfo{
			{Disposition: ffprobe.StreamDisposition{Default: 1}},
			{Disposition: ffprobe.StreamDisposition{Comment: 1}},
		},
	}

	got := BuildMuxCommand(params, "video.mkv", []string{"audio_00.mka", "audio_01.mka"})
	want := []string{
		"-hide_banner",
		"-i", "video.mkv",
		"-i", "audio_00.mka",
		"-i", "audio_01.mka",
		"-i", "input.mkv",
		"-map", "0:v:0",
		"-map", "1:a:0",
		"-map", "2:a:0",
		"-c", "copy",
		"-map_metadata", "3",
		"-map_chapters", "3",
		"-disposition:a:0", "default",
		"-disposition:a:1", "comment",
		"-movflags", "+faststart",
		"output.mkv",
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("BuildMuxCommand() = %#v, want %#v", got, want)
	}
}
