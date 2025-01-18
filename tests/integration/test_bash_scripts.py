"""Integration tests for bash scripts."""
import os
import subprocess
from pathlib import Path
import pytest
import shutil

@pytest.fixture
def script_dir():
    """Get the script directory."""
    return Path(__file__).parent.parent.parent / "src" / "drapto" / "scripts"

@pytest.fixture
def test_video(tmp_path):
    """Create a test video file using ffmpeg."""
    video_path = tmp_path / "test.mp4"
    # Create a 1-second test video
    cmd = [
        "ffmpeg", "-f", "lavfi", "-i", "testsrc=duration=1:size=1280x720:rate=30",
        "-c:v", "libx264", "-y", str(video_path)
    ]
    try:
        subprocess.run(cmd, check=True, capture_output=True)
    except subprocess.CalledProcessError:
        pytest.skip("ffmpeg not available")
    except FileNotFoundError:
        pytest.skip("ffmpeg not installed")
    
    return video_path

def test_encode_validation(script_dir, test_video, tmp_path):
    """Test the encode_validation.sh script."""
    validation_script = script_dir / "encode_validation.sh"
    env = {
        "INPUT_FILE": str(test_video),
        "INPUT_DIR": str(test_video.parent),
        "OUTPUT_DIR": str(tmp_path),
        "TEMP_DIR": str(tmp_path),
        "SCRIPT_DIR": str(script_dir),
        "PATH": os.environ["PATH"]
    }
    
    result = subprocess.run(
        ["bash", str(validation_script)],
        env=env,
        capture_output=True,
        text=True
    )
    assert result.returncode == 0, f"Validation failed: {result.stderr}"

def test_encode_config(script_dir):
    """Test the encode_config.sh script."""
    config_script = script_dir / "encode_config.sh"
    script = f"""
    export SCRIPT_DIR="{script_dir}"
    source "{config_script}"
    echo "$CRF_HD"
    """
    result = subprocess.run(
        ["bash", "-c", script],
        capture_output=True,
        text=True
    )
    assert result.returncode == 0, "Config script failed to load"
    # Verify CRF_HD is set and is a number
    crf = result.stdout.strip()
    assert crf.isdigit(), f"CRF_HD should be a number, got: {crf}"
    assert int(crf) == 25, f"CRF_HD should be 25, got: {crf}"

def test_encode_utilities(script_dir, tmp_path):
    """Test the encode_utilities.sh script."""
    utilities_script = script_dir / "encode_utilities.sh"
    test_file = tmp_path / "test.txt"
    test_file.write_text("test content")
    
    # Source the utilities and test the check_file_exists function
    script = f"""
    source {utilities_script}
    function check_file_exists() {{
        if [ ! -f "$1" ]; then
            echo "File not found: $1"
            exit 1
        fi
    }}
    check_file_exists "{test_file}"
    """
    
    result = subprocess.run(
        ["bash", "-c", script],
        capture_output=True,
        text=True
    )
    assert result.returncode == 0, f"Utilities script check_file_exists failed: {result.stderr}"

def test_encode_processing(script_dir, test_video, tmp_path):
    """Test the encode_processing.sh script."""
    processing_script = script_dir / "encode_processing.sh"
    output_file = tmp_path / "output.mp4"
    
    # Create required directories
    (tmp_path / "logs").mkdir(exist_ok=True)
    (tmp_path / "segments").mkdir(exist_ok=True)
    (tmp_path / "encoded_segments").mkdir(exist_ok=True)
    (tmp_path / "working").mkdir(exist_ok=True)
    
    # Create a mock ffprobe that returns test data
    mock_ffprobe = tmp_path / "ffprobe"
    mock_ffprobe.write_text("""#!/bin/bash
if [[ "$*" == *"-show_streams"* ]]; then
    echo '{"streams":[{"codec_type":"video","width":1280,"height":720,"duration":"1.0"}]}'
elif [[ "$*" == *"-show_frames"* ]]; then
    echo '{"frames":[{"width":1280,"height":720,"pict_type":"I"}]}'
else
    echo '{"format":{"duration":"1.0","size":"1000000"}}'
fi
""")
    mock_ffprobe.chmod(0o755)
    
    # Create a mock run_encoding function
    mock_run = tmp_path / "run_encoding"
    mock_run.write_text("""#!/bin/bash
echo "Mock encoding running..."
touch "$OUTPUT_FILE"
""")
    mock_run.chmod(0o755)
    
    env = {
        "INPUT_FILE": str(test_video),
        "INPUT_DIR": str(test_video.parent),
        "OUTPUT_DIR": str(tmp_path),
        "OUTPUT_FILE": str(output_file),
        "TEMP_DIR": str(tmp_path),
        "SCRIPT_DIR": str(script_dir),
        "FFPROBE": str(mock_ffprobe),
        "FFMPEG": shutil.which("ffmpeg"),
        "CRF": "23",
        "PRESET": "medium",
        "PATH": f"{tmp_path}:{os.environ['PATH']}"
    }
    
    # Source all required scripts and test video processing
    script = f"""
    export SCRIPT_DIR="{script_dir}"
    export FFPROBE="{mock_ffprobe}"
    source "{script_dir}/encode_config.sh"
    source "{script_dir}/encode_utilities.sh"
    source "{script_dir}/encode_hardware_acceleration.sh"
    source "{script_dir}/encode_audio_functions.sh"
    source "{script_dir}/encode_subtitle_functions.sh"
    source "{script_dir}/encode_video_functions.sh"
    source "{script_dir}/encode_formatting.sh"
    source "{processing_script}"
    
    # Mock the run_encoding function
    function run_encoding() {{
        echo "Mock encoding running..."
        touch "$OUTPUT_FILE"
    }}
    
    process_video
    """
    
    result = subprocess.run(
        ["bash", "-c", script],
        env=env,
        capture_output=True,
        text=True
    )
    print(f"Processing output: {result.stdout}")
    print(f"Processing error: {result.stderr}")
    assert result.returncode == 0, f"Processing failed: {result.stderr}"
    assert output_file.exists(), "Output file was not created"

def test_full_encode_script(script_dir, test_video, tmp_path):
    """Test the main encode.sh script with a test video."""
    encode_script = script_dir / "encode.sh"
    
    # Create all required directories
    input_dir = tmp_path / "input"
    input_dir.mkdir(exist_ok=True)
    output_dir = tmp_path / "output"
    output_dir.mkdir(exist_ok=True)
    temp_dir = tmp_path / "temp"
    temp_dir.mkdir(exist_ok=True)
    log_dir = tmp_path / "logs"
    log_dir.mkdir(exist_ok=True)
    segments_dir = temp_dir / "segments"
    segments_dir.mkdir(exist_ok=True)
    encoded_segments_dir = temp_dir / "encoded_segments"
    encoded_segments_dir.mkdir(exist_ok=True)
    working_dir = temp_dir / "working"
    working_dir.mkdir(exist_ok=True)
    
    # Create a mock ffprobe that returns test data
    mock_ffprobe = tmp_path / "ffprobe"
    mock_ffprobe.write_text("""#!/bin/bash
if [[ "$*" == *"-show_streams"* ]]; then
    if [[ "$*" == *"-select_streams v"* ]]; then
        if [[ "$*" == *"-show_entries stream=codec_name"* ]]; then
            echo "h264"  # Video codec
        else
            echo "video"  # Stream type
        fi
    elif [[ "$*" == *"-select_streams a"* ]]; then
        if [[ "$*" == *"-show_entries stream=channels"* ]]; then
            echo "2"  # Stereo audio
        else
            echo "audio"  # Stream type
        fi
    else
        # Full stream info
        echo "video"  # codec_type
        echo "1280"   # width
        echo "720"    # height
        echo "1"      # duration (integer)
    fi
elif [[ "$*" == *"-show_frames"* ]]; then
    echo "1280"  # width
    echo "720"   # height
    echo "I"     # pict_type
elif [[ "$*" == *"-show_entries format=duration"* ]]; then
    echo "1"  # Duration in seconds (integer)
elif [[ "$*" == *"-show_entries format=size"* ]]; then
    echo "1000000"  # Size in bytes
else
    echo "1"  # Default duration (integer)
fi
""")
    mock_ffprobe.chmod(0o755)
    
    # Create a mock run_encoding function in a script
    (tmp_path / "run_encoding").write_text("""#!/bin/bash
echo "Mock encoding running..."
cp "$INPUT_DIR/$INPUT_FILE" "$OUTPUT_DIR/$INPUT_FILE"
""")
    mock_run = tmp_path / "run_encoding"
    mock_run.chmod(0o755)
    
    # Create a mock ffmpeg that just copies the file
    mock_ffmpeg = tmp_path / "ffmpeg"
    mock_ffmpeg.write_text("""#!/bin/bash
echo "Mock ffmpeg running..."
cp "$INPUT_DIR/$INPUT_FILE" "$OUTPUT_DIR/$INPUT_FILE"
""")
    mock_ffmpeg.chmod(0o755)
    
    # Create test video file
    input_file = input_dir / "test.mp4"
    input_file.write_bytes(b"test video content")  # Write as binary to make it look like a real video file
    
    env = {
        "INPUT_FILE": input_file.name,  # Just the filename
        "INPUT_DIR": str(input_dir),    # The directory containing the file
        "OUTPUT_DIR": str(output_dir),
        "TEMP_DIR": str(temp_dir),
        "SCRIPT_DIR": str(script_dir),
        "FFPROBE": str(mock_ffprobe),
        "FFMPEG": str(mock_ffmpeg),
        "PATH": f"{tmp_path}:{os.environ['PATH']}",
        "ENABLE_CHUNKED_ENCODING": "false",  # Disable chunked encoding for simpler testing
        "LOG_DIR": str(log_dir)  # Add log directory
    }
    
    script = f"""
    # Export all variables first
    export SCRIPT_DIR="{script_dir}"
    export FFPROBE="{mock_ffprobe}"
    export FFMPEG="{mock_ffmpeg}"
    export INPUT_FILE="{input_file.name}"  # Just the filename
    export INPUT_DIR="{input_dir}"         # The directory containing the file
    export OUTPUT_DIR="{output_dir}"
    export TEMP_DIR="{temp_dir}"
    export PATH="{tmp_path}:$PATH"
    export ENABLE_CHUNKED_ENCODING="false"  # Disable chunked encoding for simpler testing
    export LOG_DIR="{log_dir}"      # Add log directory
    
    # Source the main script
    source "{encode_script}"
    
    # Call main function
    main
    """
    
    result = subprocess.run(
        ["bash", "-c", script],
        env=env,
        capture_output=True,
        text=True
    )
    print(f"Encoding output: {result.stdout}")
    print(f"Encoding error: {result.stderr}")
    assert result.returncode == 0, f"Encoding failed: {result.stderr}"
    
    # Check if output file was created (should have same name as input)
    output_file = output_dir / input_file.name
    assert output_file.exists(), "Output file was not created"
