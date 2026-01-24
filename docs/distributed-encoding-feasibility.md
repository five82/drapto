# Distributed Encoding Feasibility Analysis

This document analyzes the feasibility of true distributed encoding across multiple machines with shared NFS storage, based on the existing `chunkencoding` branch architecture.

## Executive Summary

**Verdict: Feasible with moderate architectural changes**

The existing chunked encoding pipeline provides an excellent foundation for distributed encoding. The key insight is that chunks are already independent units of work with clear boundaries. The main challenges are coordinating chunk assignment, handling partial failures, and managing NFS performance characteristics.

## Current Architecture Review

The `chunkencoding` branch implements:

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Indexing   │────▶│   Chunking   │────▶│  Dispatching │
│   (FFMS2)    │     │  (fixed-len) │     │  (in-memory) │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                     ┌────────────────────────────┼────────────────────────────┐
                     │                            │                            │
                     ▼                            ▼                            ▼
              ┌────────────┐               ┌────────────┐               ┌────────────┐
              │  Worker 1  │               │  Worker 2  │               │  Worker N  │
              │  (goroutine)│               │  (goroutine)│               │  (goroutine)│
              │  decode→enc │               │  decode→enc │               │  decode→enc │
              └─────┬──────┘               └─────┬──────┘               └─────┬──────┘
                    │                            │                            │
                    ▼                            ▼                            ▼
              ┌────────────┐               ┌────────────┐               ┌────────────┐
              │  0000.ivf  │               │  0001.ivf  │               │  NNNN.ivf  │
              └────────────┘               └────────────┘               └────────────┘
                    │                            │                            │
                    └────────────────────────────┼────────────────────────────┘
                                                 │
                                                 ▼
                                          ┌────────────┐
                                          │   Merge    │
                                          │  (FFmpeg)  │
                                          └────────────┘
```

### Key Characteristics

1. **Independent chunks**: Each chunk encodes independently with no inter-chunk dependencies
2. **Streaming pipeline**: Single-frame decode→encode minimizes memory per worker (~1GB)
3. **Resume support**: `done.txt` tracks completed chunks for crash recovery
4. **CGO dependency**: FFMS2 bindings require compiled library on each node

---

## Distributed Architecture Proposal

### Option A: Coordinator-Based Distribution

```
                              ┌─────────────────┐
                              │   Coordinator   │
                              │   (single node) │
                              │                 │
                              │ - Chunk dispatch│
                              │ - Progress track│
                              │ - Merge control │
                              └────────┬────────┘
                                       │ gRPC/HTTP
            ┌──────────────────────────┼──────────────────────────┐
            │                          │                          │
            ▼                          ▼                          ▼
     ┌─────────────┐           ┌─────────────┐           ┌─────────────┐
     │   Node A    │           │   Node B    │           │   Node C    │
     │  (encoder)  │           │  (encoder)  │           │  (encoder)  │
     │             │           │             │           │             │
     │ - FFMS2     │           │ - FFMS2     │           │ - FFMS2     │
     │ - SVT-AV1   │           │ - SVT-AV1   │           │ - SVT-AV1   │
     │ - Local     │           │ - Local     │           │ - Local     │
     │   workers   │           │   workers   │           │   workers   │
     └──────┬──────┘           └──────┬──────┘           └──────┬──────┘
            │                          │                          │
            └──────────────────────────┼──────────────────────────┘
                                       │
                                       ▼
                              ┌─────────────────┐
                              │   NFS Share     │
                              │                 │
                              │ /source/        │
                              │ /work/encode/   │
                              │ /output/        │
                              └─────────────────┘
```

**Pros:**
- Centralized state management simplifies consistency
- Natural fit with Spindle's orchestration model
- Easy to add/remove nodes dynamically

**Cons:**
- Coordinator is single point of failure
- Additional network round-trips for chunk assignment

### Option B: File-Based Coordination (Lock-Free)

```
┌─────────────────────────────────────────────────────────────────────┐
│                           NFS Share                                  │
│                                                                      │
│  /encode-jobs/                                                       │
│    job-12345/                                                        │
│      source.mkv          ← Input video                               │
│      chunks.json         ← Chunk definitions (read-only after init)  │
│      claims/             ← Lock-free chunk claiming                  │
│        0000.claimed      ← Contains: node-id, timestamp              │
│        0001.claimed                                                  │
│        0002.claimed                                                  │
│      encode/             ← Output IVF files                          │
│        0000.ivf                                                      │
│        0001.ivf                                                      │
│      done/               ← Completion markers                        │
│        0000.done         ← Contains: frames, size, checksum          │
│        0001.done                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

Workers use atomic file operations to claim chunks:

```go
// Claim a chunk using O_EXCL (atomic create)
func claimChunk(jobDir string, chunkIdx int, nodeID string) (bool, error) {
    claimPath := filepath.Join(jobDir, "claims", fmt.Sprintf("%04d.claimed", chunkIdx))

    f, err := os.OpenFile(claimPath, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0644)
    if os.IsExist(err) {
        return false, nil // Already claimed
    }
    if err != nil {
        return false, err
    }
    defer f.Close()

    fmt.Fprintf(f, "%s\n%d\n", nodeID, time.Now().Unix())
    return true, nil
}
```

**Pros:**
- No coordinator needed
- Naturally fault-tolerant
- NFS provides the distributed state

**Cons:**
- Relies on NFS atomic operations (O_EXCL)
- Stale claim detection requires timeout logic
- Less visibility into cluster state

---

## Key Technical Challenges

### 1. FFMS2 Index Management

**Problem**: FFMS2 creates a `.ffindex` file that enables frame-accurate seeking. Creating this index is expensive (comparable to a full decode pass).

**Options**:

| Approach | Pros | Cons |
|----------|------|------|
| **Shared index on NFS** | Index created once, reused by all nodes | Index file is node-specific (paths embedded), may not be portable |
| **Per-node indexing** | Each node has clean local index | Duplicated work, wasted time |
| **Pre-compute + embed** | Coordinator indexes, embeds in job manifest | Additional complexity |
| **Use FFmpeg instead** | Avoid FFMS2 dependency | Lose frame-accurate seeking, must use keyframe alignment |

**Recommendation**: Test NFS index portability. If indices are portable (paths are relative), shared indexing is ideal. Otherwise, per-node indexing with local caching.

### 2. NFS Performance for Video I/O

**Read patterns during encoding:**
- Source video: Sequential reads with occasional seeks (for chunk start positions)
- Write patterns: Sequential writes of IVF chunks

**Performance considerations:**

| Factor | Impact | Mitigation |
|--------|--------|------------|
| **Latency** | Frame extraction involves many small reads | FFMS2 uses read-ahead buffering |
| **Bandwidth** | Source reads × N nodes could saturate network | Stagger chunk starts, local caching |
| **Write contention** | Multiple nodes writing to same directory | Use per-chunk files (already done) |
| **NFS cache coherency** | Workers might see stale directory listings | Use fsync, direct I/O for coordination files |

**Bandwidth calculation example:**
- 4K source at 50 Mbps × 4 nodes = 200 Mbps sustained read (25 MB/s)
- Typical 1 Gbps NFS can handle ~100 MB/s, so 4 nodes is comfortable
- 10 Gbps would support 10+ encoding nodes

### 3. Failure Handling

**Scenarios to handle:**

| Failure Mode | Detection | Recovery |
|--------------|-----------|----------|
| Node dies mid-encode | Stale claim (no heartbeat/completion) | Timeout + re-claim by another node |
| Network partition | Node continues but can't write | Claim expires, chunk re-encoded |
| Corrupt output | Invalid IVF file | Checksum verification + re-encode |
| NFS unavailable | All nodes stall | Wait + retry with backoff |

**Claim timeout strategy:**
```go
const (
    claimTimeout    = 30 * time.Minute  // Max time to encode one chunk
    heartbeatPeriod = 1 * time.Minute   // Update claim timestamp
)

func isClaimStale(claimPath string) bool {
    info, _ := os.Stat(claimPath)
    return time.Since(info.ModTime()) > claimTimeout
}
```

### 4. Load Balancing

**Problem**: Chunks vary in complexity. Simple scenes encode faster than complex ones.

**Current approach**: Dispatcher prioritizes chunks adjacent to completed ones (for CRF prediction in TQ mode).

**Distributed options:**

| Strategy | Implementation | Trade-offs |
|----------|---------------|------------|
| **Work stealing** | Nodes claim smallest unclaimed chunk | Balances load, more contention |
| **Predicted complexity** | Pre-analyze scenes, assign by complexity | Extra analysis pass |
| **Chunking by duration** | Fixed-time chunks already implemented | Good enough for most cases |

---

## Proposed Implementation Phases

### Phase 1: Shared Work Directory Support

Modify existing pipeline to support NFS work directories:

```go
type DistributedConfig struct {
    NodeID        string        // Unique identifier for this node
    WorkDir       string        // NFS path to shared work directory
    LocalTempDir  string        // Local SSD for scratch space
    ClaimTimeout  time.Duration // Stale claim threshold
}
```

**Changes required:**
- Add node ID to claim files
- Implement claim-based chunk selection
- Add heartbeat/progress updates to claim files
- Handle stale claim detection and re-claiming

### Phase 2: Multi-Node Coordination

Add lightweight coordination service (or use file-based coordination):

```go
// Coordinator interface - can be backed by gRPC service or file system
type ChunkCoordinator interface {
    // ClaimChunk attempts to claim the next available chunk
    ClaimChunk(ctx context.Context, nodeID string) (*Chunk, error)

    // CompleteChunk marks a chunk as done
    CompleteChunk(ctx context.Context, chunkIdx int, result ChunkResult) error

    // HeartbeatChunk updates claim timestamp
    HeartbeatChunk(ctx context.Context, chunkIdx int) error

    // GetProgress returns overall job progress
    GetProgress(ctx context.Context) (*JobProgress, error)
}
```

### Phase 3: Integration with Spindle

Spindle already orchestrates encoding jobs. Adding distributed encoding support:

```go
// In Spindle's encoding phase
type DistributedEncodeRequest struct {
    SourcePath  string   // NFS path to source
    OutputPath  string   // NFS path for output
    Nodes       []string // Available encoding nodes
    WorkDir     string   // NFS shared work directory
}

// Spindle coordinates:
// 1. Creates job directory with chunks.json
// 2. Notifies nodes to start encoding
// 3. Monitors progress via done/ directory
// 4. Triggers merge when all chunks complete
```

---

## Benefits

### Performance Scaling

| Nodes | Estimated Speedup | Notes |
|-------|-------------------|-------|
| 1 | 1.0× (baseline) | Current implementation |
| 2 | ~1.8× | Near-linear with fixed overhead |
| 4 | ~3.4× | Coordination overhead increases |
| 8 | ~5.5× | NFS bandwidth may become bottleneck |

*Assumes 10 Gbps NFS, 4K source, 8-core nodes*

### Resource Utilization

- **Idle machines contribute**: Workstations can join the encode pool when unused
- **Heterogeneous hardware**: Mix of powerful and modest machines works fine
- **Cloud burst**: Spin up cloud instances during peak load

### Fault Tolerance

- **Node failure**: Other nodes pick up abandoned chunks
- **Graceful degradation**: Fewer nodes = slower, but still completes
- **Resume anywhere**: Any node can resume a partially-complete job

---

## Drawbacks

### Complexity

| Aspect | Single-Node | Distributed |
|--------|-------------|-------------|
| Setup | Install drapto | Install on all nodes, configure NFS |
| Debugging | Local logs | Aggregated logs across nodes |
| State management | done.txt | Distributed claims + coordination |
| Failure modes | Process crash | Network partitions, stale claims |

### NFS Dependencies

- **Performance variance**: NFS latency affects all nodes
- **Single point of failure**: NFS outage stops all encoding
- **Configuration complexity**: Proper NFS tuning required

### Additional Overhead

- **Coordination**: ~1-5% overhead for claim management
- **Network**: Source reads multiplied by node count
- **Indexing**: If not shared, duplicated per node

---

## Recommendation

**Start with Option B (file-based coordination)** for simplicity:

1. Chunks are defined in a JSON file on NFS
2. Workers atomically claim chunks using O_EXCL
3. Completion is marked by writing `.done` files
4. Any node can trigger merge once all `.done` files exist

This approach:
- Requires minimal changes to existing code
- Works with any number of nodes
- Has no coordinator to fail
- Integrates naturally with Spindle (which already monitors file system)

**Future enhancement**: If coordination needs grow complex, add a gRPC coordinator service (possibly as part of Spindle itself).

---

## Prototype Outline

```go
// cmd/drapto-worker/main.go - Distributed encoding worker

func main() {
    cfg := loadConfig()

    for {
        // Find unclaimed chunk
        chunk, err := claimNextChunk(cfg.JobDir, cfg.NodeID)
        if err == ErrNoChunksAvailable {
            log.Info("All chunks claimed or complete, exiting")
            break
        }

        // Start heartbeat goroutine
        ctx, cancel := context.WithCancel(context.Background())
        go heartbeat(ctx, cfg.JobDir, chunk.Idx)

        // Encode chunk (reuses existing encode logic)
        result, err := encodeChunk(cfg.JobDir, chunk)
        cancel() // Stop heartbeat

        if err != nil {
            log.Error("Chunk failed", "chunk", chunk.Idx, "err", err)
            releaseClaim(cfg.JobDir, chunk.Idx)
            continue
        }

        // Mark complete
        markComplete(cfg.JobDir, chunk.Idx, result)
    }
}
```

---

## Conclusion

Distributed encoding across NFS is **feasible** with the existing chunked architecture. The key enablers are:

1. **Chunks are independent** - No inter-chunk dependencies
2. **Resume support exists** - Progress tracking via done.txt translates to distributed claims
3. **IVF files are portable** - Any node can write, any node can merge

The main challenges are coordination (solved with atomic file operations) and NFS performance (mitigated by proper network provisioning).

For a hobby project, the file-based coordination approach provides 80% of the benefit with 20% of the complexity. A full coordinator service could be added later if needed.
