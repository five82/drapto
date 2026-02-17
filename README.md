# drapto

FFmpeg wrapper for AV1 encoding with SVT-AV1 and Opus audio. Provides opinionated defaults, automatic crop detection, HDR preservation, and post-encode validation.

## Expectations

This repository is shared as is. Drapto is a personal encoding tool I built for my own workflow, hardware, and preferences. I've open sourced it because I believe in sharing but I'm not an active maintainer.

- Personal-first: Things will change and break as I iterate.
- Best-effort only: This is a part-time hobby project and I work on it when I'm able to. I may be slow to respond to questions or may not respond at all.
- PRs: Pull requests are welcome if they align with the project's goals but I may be slow to review them or may not accept changes that don't fit my own use case.
- “Vibe coded”: I’m not a Go developer and this project started as (and remains) a vibe-coding experiment. Expect rough edges.

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
