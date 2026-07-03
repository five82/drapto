# AGENTS.md

## TL;DR

- Do not run `git commit` or `git push` unless explicitly instructed
- Run `./check-ci.sh` before handing work back

## Project

Drapto is an FFmpeg wrapper for AV1 encoding. Single-developer hobby project.

## Commands

```bash
go build -o drapto ./cmd/drapto
go test ./...
golangci-lint run
./check-ci.sh
```

## Principles

1. Keep it simple - small hobby project
2. Prefer unit tests over actual encodes
3. Use 120s+ timeout when running drapto
