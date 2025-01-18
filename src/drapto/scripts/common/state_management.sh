#!/usr/bin/env bash

###################
# State Management
###################

# Import common utilities
source "${SCRIPT_DIR}/utils/formatting.sh"

# Create a new encoding job
# Args:
#   $1: Input file path
#   $2: Output file path
#   $3: Strategy name
create_encoding_job() {
    local input_file="$1"
    local output_file="$2"
    local strategy="$3"
    
    local job_id
    job_id=$(PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
state = EncodingState(\"${TEMP_DATA_DIR}\")
job_id = state.create_job(\"${input_file}\", \"${output_file}\", \"${strategy}\")
print(job_id)
")
    echo "$job_id"
}

# Add a new segment to a job
# Args:
#   $1: Job ID
#   $2: Segment index
#   $3: Input segment path
#   $4: Output segment path
#   $5: Start time (optional)
#   $6: Duration (optional)
add_segment() {
    local job_id="$1"
    local index="$2"
    local input_path="$3"
    local output_path="$4"
    local start_time="${5:-0.0}"
    local duration="${6:-0.0}"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.add_segment(\"${job_id}\", ${index}, \"${input_path}\", \"${output_path}\", ${start_time}, ${duration})
"
}

# Update segment status
# Args:
#   $1: Job ID
#   $2: Segment index
#   $3: Status (pending, encoding, completed, failed)
#   $4: Error message (optional)
update_segment_status() {
    local job_id="$1"
    local index="$2"
    local status="$3"
    local error_msg="$4"
    
    if [[ -n "$error_msg" ]]; then
        PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState, SegmentStatus
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_segment_status(\"${job_id}\", ${index}, SegmentStatus(\"${status}\"), \"${error_msg}\")
"
    else
        PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState, SegmentStatus
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_segment_status(\"${job_id}\", ${index}, SegmentStatus(\"${status}\"))
"
    fi
}

# Get all segments for a job
# Args:
#   $1: Job ID
get_segments() {
    local job_id="$1"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
segments = state.get_segments(\"${job_id}\")
print(json.dumps([s.__dict__ for s in segments], default=str))
"
}

# Get a specific segment
# Args:
#   $1: Job ID
#   $2: Segment index
get_segment() {
    local job_id="$1"
    local index="$2"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
segment = state.get_segment(\"${job_id}\", ${index})
print(json.dumps(segment.__dict__, default=str))
"
}

# Update job status
# Args:
#   $1: Job ID
#   $2: Status (pending, initializing, preparing, encoding, finalizing, completed, failed)
#   $3: Error message (optional)
update_job_status() {
    local job_id="$1"
    local status="$2"
    local error_msg="$3"
    
    if [[ -n "$error_msg" ]]; then
        PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState, JobStatus
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_job_status(\"${job_id}\", JobStatus(\"${status}\"), \"${error_msg}\")
"
    else
        PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState, JobStatus
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_job_status(\"${job_id}\", JobStatus(\"${status}\"))
"
    fi
}

# Update job statistics
# Args:
#   $1: Job ID
#   $2: Input size (bytes)
#   $3: Output size (bytes)
#   $4: VMAF score
update_job_stats() {
    local job_id="$1"
    local input_size="$2"
    local output_size="$3"
    local vmaf_score="$4"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_job_stats(
    \"${job_id}\",
    input_size=${input_size},
    output_size=${output_size},
    vmaf_score=${vmaf_score}
)
"
}

# Get job information
# Args:
#   $1: Job ID
get_job_info() {
    local job_id="$1"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
job = state.get_job(\"${job_id}\")
print(json.dumps(job.__dict__, default=str))
"
}

# Get all jobs information
get_all_jobs() {
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
jobs = state.get_all_jobs()
print(json.dumps([job.__dict__ for job in jobs], default=str))
"
}

# Update job progress
# Args:
#   $1: Job ID
#   $2: Current frame
#   $3: Total frames
#   $4: FPS (optional)
update_job_progress() {
    local job_id="$1"
    local current_frame="$2"
    local total_frames="$3"
    local fps="${4:-0.0}"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_job_progress(\"${job_id}\", ${current_frame}, ${total_frames}, ${fps})
"
}

# Update segment progress
# Args:
#   $1: Job ID
#   $2: Segment index
#   $3: Current frame
#   $4: Total frames
#   $5: FPS (optional)
update_segment_progress() {
    local job_id="$1"
    local index="$2"
    local current_frame="$3"
    local total_frames="$4"
    local fps="${5:-0.0}"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
state = EncodingState(\"${TEMP_DATA_DIR}\")
state.update_segment_progress(\"${job_id}\", ${index}, ${current_frame}, ${total_frames}, ${fps})
"
}

# Get job progress
# Args:
#   $1: Job ID
get_job_progress() {
    local job_id="$1"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
progress = state.get_progress(\"${job_id}\")
print(json.dumps(progress.__dict__, default=str))
"
}

# Get segment progress
# Args:
#   $1: Job ID
#   $2: Segment index
get_segment_progress() {
    local job_id="$1"
    local index="$2"
    
    PYTHONPATH="$HOME/projects/encodeworkflow/python/drapto/src" python3 -c "
from drapto.scripts.common.encoding_state import EncodingState
import json
state = EncodingState(\"${TEMP_DATA_DIR}\")
progress = state.get_segment_progress(\"${job_id}\", ${index})
print(json.dumps(progress.__dict__, default=str))
"
}
