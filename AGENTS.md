# AGENTS.md

This file provides guidance when working with code in this repository.

CLAUDE.md and GEMINI.md are symlinks to this file so all agent guidance stays in one place.
Do not modify this header.

## TL;DR

- Do not run `git commit` or `git push` unless explicitly instructed.
- Run `./check-ci.sh` before handing work back.
- Use Context7 MCP for library/API docs without being asked.

## Project Snapshot

Drapto is an **FFmpeg wrapper** for AV1 encoding with SVT-AV1 and Opus audio. It provides opinionated defaults, automatic crop detection, HDR preservation, and post-encode validation.

- **Scope**: Single-developer hobby project - avoid over-engineering
- **Environment**: Go 1.25+, FFmpeg (libsvtav1, libopus), MediaInfo
- **Design**: Library-first for Spindle embedding, with CLI wrapper

## Related Repos

| Repo | Path | Role |
|------|------|------|
| drapto | `~/projects/drapto/` | FFmpeg encoding wrapper (this repo) |
| spindle | `~/projects/spindle/` | Orchestrator that shells out to Drapto during ENCODING |
| flyer | `~/projects/flyer/` | Read-only TUI for Spindle (not a Drapto consumer) |

GitHub: [drapto](https://github.com/five82/drapto) | [spindle](https://github.com/five82/spindle) | [flyer](https://github.com/five82/flyer)

## Build, Test, Lint

```bash
go build -o drapto ./cmd/drapto       # Build CLI
go test ./...                         # Test
go test -race ./...                   # Race detector
golangci-lint run                     # Lint
./check-ci.sh                         # Full CI (recommended before handoff)
```

## Architecture

```
drapto.go, events.go, reporter.go  # Public API: Encoder, Options, EventHandler, Reporter
cmd/drapto/main.go                 # CLI wrapper (standard flag package)
internal/
├── config/              # Configuration and presets
├── ffmpeg/              # FFmpeg command builder + executor
├── ffprobe/             # Media analysis
├── mediainfo/           # HDR detection
├── processing/          # Orchestrator, crop detection, audio
├── validation/          # Post-encode validation checks
├── reporter/            # Progress: Terminal, Composite, Log
├── discovery/           # Video file discovery
├── logging/             # File logging
└── util/                # Formatting, file, temp file utils
```

## Entry Points

| Task | Start Here |
|------|------------|
| Encoding parameters | `internal/config/config.go`, `internal/ffmpeg/command.go` |
| Crop detection | `internal/processing/crop.go` |
| Validation checks | `internal/validation/validate.go` |
| Terminal output | `internal/reporter/terminal.go` |
| HDR detection | `internal/mediainfo/mediainfo.go`, `internal/ffprobe/ffprobe.go` |
| Public API | `drapto.go` |
| CLI flags | `cmd/drapto/main.go` |

## CLI Output Style

1. Five sections in human mode: Hardware → Video → Encoding → Validation → Results
2. Colors via `fatih/color`, progress via `schollz/progressbar`
3. Show progress info once (progress bar during encode, summary after validation)
4. Natural language sentences; emphatic formatting for key values only

## Spindle Integration

See `docs/spindle-integration.md` for library API usage and event types.

**Versioning workflow:**
- Spindle references drapto via `go.mod`. During active development, it uses `@main` (pseudo-versions).
- After pushing drapto changes, update spindle with `go get github.com/five82/drapto@main`.
- Formal version tags (v1.0.0, etc.) are deferred until the API stabilizes.
- Breaking API changes are fine - just update spindle in the same session.

## Principles

1. Keep it simple - small hobby project
2. Prefer unit tests over actual encodes (encoding is slow)
3. When running drapto with timeout, use at least 120 seconds
