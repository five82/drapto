# Drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Provides opinionated defaults, automatic crop detection, HDR preservation, and post-encode validation.

## Requirements

- Go 1.26+
- FFmpeg with libsvtav1 and libopus
- MediaInfo

## Install

```bash
go install github.com/five82/drapto/cmd/drapto@latest
```

## Usage

```bash
drapto encode -i input.mkv -o output/
drapto encode -i /videos/ -o /encoded/ --drapto-preset grain
drapto --help
```

## License

GPL-3.0
