"""Segment merging functionality

Responsibilities:
- Merge short video segments
- Validate merged segment output
- Handle cleanup of temporary merge files
"""

import logging
from pathlib import Path
from typing import List

from ..utils import run_cmd
from ..exceptions import SegmentMergeError
from ..ffprobe.utils import get_duration, MetadataError

logger = logging.getLogger(__name__)

def merge_segments(segments: List[Path], output: Path) -> None:
    """
    Merge video segments using ffmpeg's concat demuxer
    
    Args:
        segments: List of segment paths to merge
        output: Output path for merged segment
        
    Raises:
        SegmentMergeError: If merging fails
    """
    # Create temporary concat file
    concat_file = output.parent / "concat.txt"
    try:
        with open(concat_file, 'w') as f:
            for segment in segments:
                f.write(f"file '{segment.absolute()}'\n")
            
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "warning",
            "-f", "concat",
            "-safe", "0",
            "-i", str(concat_file),
            "-c", "copy",
            "-y", str(output)
        ]
        run_cmd(cmd)
        
        # Verify merged output
        if not output.exists() or output.stat().st_size == 0:
            logger.error("Failed to create merged segment")
            raise SegmentMergeError("Failed to create merged segment", module="segmentation")
            
    except Exception as e:
        logger.error("Failed to merge segments: %s", e)
        raise SegmentMergeError(f"Failed to merge segments: {str(e)}", module="segmentation") from e
    finally:
        if concat_file.exists():
            concat_file.unlink()
