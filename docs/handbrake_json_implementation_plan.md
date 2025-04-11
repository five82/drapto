# Drapto: HandBrakeCLI JSON Parsing Implementation Plan

---

## Goal

Integrate robust parsing of HandBrakeCLI's `--json` output into Drapto, to improve metadata extraction, progress tracking, and error handling during **encoding** and **film grain detection**.

---

## High-Level Steps

1. **Add `--json` to all HandBrakeCLI invocations**
2. **Capture and buffer stdout**
3. **Extract multiple JSON snippets from mixed output**
4. **Define Rust data structures with serde**
5. **Parse and dispatch JSON snippets**
6. **Integrate parsed data into workflows**
7. **Testing**

---

## Step-by-Step Instructions

### 1. Add `--json` to HandBrakeCLI invocations

- **Encoding:**  
  In `drapto-core/src/processing/video.rs`, when building the HandBrakeCLI command, **append `--json`** to the argument list.

- **Film grain detection:**  
  In `drapto-core/src/processing/film_grain/`, locate any HandBrakeCLI calls and **add `--json`** similarly.

- **Optional:**  
  Make JSON mode configurable via CLI flag or config file.

---

### 2. Capture and buffer stdout

- Instead of streaming stdout line-by-line to logs, **buffer the entire stdout output** of HandBrakeCLI **until process exit**.
- You can still stream stderr for real-time error logging.
- After process exit, **split the buffered stdout into lines** for JSON extraction.

---

### 3. Extract multiple JSON snippets

- HandBrakeCLI outputs **multiple standalone JSON objects** interleaved with plain text.
- Implement a **JSON snippet extractor** that:
  - Scans lines for those starting with `{` or a known JSON key (e.g., `"Version":`, `"Progress":`, `"Audio":`)
  - Collects lines until a **balanced closing `}`** is found
  - Ignores non-JSON lines
- Alternatively, use a **stack-based brace counter** to extract well-formed JSON blocks.

---

### 4. Define Rust data structures

- Use `serde` with `#[derive(Deserialize)]`.
- Define **enums or tagged structs** for different snippet types:

```rust
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum HandBrakeJsonSnippet {
    Version(VersionInfo),
    Progress(ProgressInfo),
    JobConfig(JobConfig),
    // Add other types as needed
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo { /* ... */ }

#[derive(Debug, Deserialize)]
pub struct ProgressInfo { /* ... */ }

#[derive(Debug, Deserialize)]
pub struct JobConfig { /* ... */ }
```

- Refer to `docs/handbrake_json_example.txt` for schema details.

---

### 5. Parse and dispatch JSON snippets

- For each extracted JSON snippet:
  - Attempt to deserialize into `HandBrakeJsonSnippet`.
  - **Dispatch** based on the variant:
    - **Version:** Save version info
    - **Progress:** Update progress UI/state
    - **JobConfig:** Save input metadata, crop info, filters, etc.
- Handle parse errors gracefully (log and continue).

---

### 6. Integrate parsed data into workflows

- **Encoding pipeline:**
  - Use **JobConfig** metadata to replace or augment ffprobe info.
  - Use **Progress** updates for real-time CLI/UI feedback.
  - Detect errors or warnings from JSON status.

- **Film grain detection:**
  - Use scan metadata (crop, resolution, color) from JSON.
  - Track progress if needed.

- **Notifications:**
  - Use parsed progress and status to send more informative notifications.

- **Error handling:**
  - Detect failures from JSON, not just exit codes or stderr.

---

### 7. Testing

- Create **unit tests** for the JSON snippet extractor with mixed output samples.
- Add **serde deserialization tests** using `handbrake_json_example.txt`.
- Add **integration tests** for encoding and film grain detection with `--json` enabled.
- Verify:
  - All JSON snippets are correctly extracted and parsed.
  - Progress updates are accurate.
  - Metadata matches expectations.
  - Errors are detected reliably.

---

## Additional Notes

- **Performance:**  
  Buffering stdout may increase memory use, but JSON snippets are small.

- **Compatibility:**  
  If `--json` is not supported in some HandBrakeCLI versions, detect and fallback gracefully.

- **Extensibility:**  
  The enum approach allows adding new snippet types easily.

---

## Status

This is a **detailed implementation plan**.