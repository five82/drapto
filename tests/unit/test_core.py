"""Unit tests for core functionality."""
import os
import subprocess
import queue
from pathlib import Path
import pytest
from drapto.core import Encoder

def test_encoder_initialization():
    """Test encoder initialization."""
    encoder = Encoder()
    assert encoder.script_dir.exists()
    assert encoder.encode_script.exists()
    assert os.access(encoder.encode_script, os.X_OK)
    # Verify color environment setup
    assert encoder.env["FORCE_COLOR"] == "1"
    assert encoder.env["CLICOLOR"] == "1"
    assert encoder.env["CLICOLOR_FORCE"] == "1"
    assert encoder.env["NO_COLOR"] == "0"
    assert encoder.env["COLORTERM"] == "truecolor"
    assert "TERM" in encoder.env

def test_encoder_with_invalid_script_dir(tmp_path):
    """Test encoder initialization with invalid script dir."""
    class BadEncoder(Encoder):
        def __init__(self):
            # Set script_dir before super().__init__() so the check happens
            self.script_dir = tmp_path / "nonexistent"
            super().__init__()  # This will now raise the error

    with pytest.raises(RuntimeError, match=f"Script directory not found: {tmp_path / 'nonexistent'}"):
        BadEncoder()

def test_stream_reader(encoder, mocker):
    """Test the stream reader functionality."""
    mock_stream = mocker.MagicMock()
    mock_stream.readline.side_effect = [
        "line1\n",
        "line2\n",
        ""  # EOF
    ]
    test_queue = queue.Queue()
    
    # Run stream reader
    encoder._stream_reader(mock_stream, test_queue, 'test')
    
    # Check output
    assert test_queue.get() == ('test', 'line1')
    assert test_queue.get() == ('test', 'line2')
    assert test_queue.get() == ('test', None)  # EOF marker
    assert mock_stream.close.called

def test_encode_file_environment_setup(encoder, mock_video_file, temp_dir, mocker):
    """Test environment setup for encoding."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen for process management
    mock_process = mocker.MagicMock()
    mock_process.returncode = 0
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    mock_process.poll.return_value = 0  # Process completes immediately

    # Mock queue for output handling
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = [
        ('stdout', 'Processing...'),
        queue.Empty(),  # Simulate timeout
    ]

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")  # Mock thread creation
    mocker.patch("time.sleep")  # Speed up test

    # Call the internal encode method
    encoder._encode_file(mock_video_file, output_path, env)

    # Verify environment setup
    mock_popen = subprocess.Popen
    mock_popen.assert_called_once()
    call_env = mock_popen.call_args[1]["env"]
    assert "INPUT_FILE" in call_env
    assert "OUTPUT_FILE" in call_env
    assert "INPUT_DIR" in call_env
    assert "OUTPUT_DIR" in call_env
    assert "LOG_DIR" in call_env
    assert "TEMP_DATA_DIR" in call_env
    assert call_env["FORCE_COLOR"] == "1"
    assert call_env["CLICOLOR"] == "1"
    assert call_env["CLICOLOR_FORCE"] == "1"

def test_encode_file_process_cleanup(encoder, mock_video_file, temp_dir, mocker):
    """Test process cleanup on error."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen for process management
    mock_process = mocker.MagicMock()
    mock_process.returncode = None  # Process hasn't finished
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    mock_process.poll.side_effect = [None, None]  # Process still running
    mock_process.wait.side_effect = [subprocess.TimeoutExpired("cmd", 2), 0]  # Timeout then success
    mock_process.kill.return_value = None  # Kill succeeds silently

    # Mock queue that raises KeyboardInterrupt
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = KeyboardInterrupt()

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")  # Mock thread creation
    mocker.patch("time.sleep")  # Speed up test

    # Call the internal encode method - should handle the interrupt
    with pytest.raises(KeyboardInterrupt):
        encoder._encode_file(mock_video_file, output_path, env)

    # Verify cleanup - process should be terminated and then killed
    assert mock_process.terminate.call_count >= 1
    assert mock_process.kill.call_count >= 1

def test_encode_file_output_handling(encoder, mock_video_file, temp_dir, mocker):
    """Test handling of process output."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen with some typical ffmpeg output
    mock_process = mocker.MagicMock()
    mock_process.returncode = None  # Start with no return code
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    mock_process.poll.side_effect = [None, None, 0]  # Process completes after output
    mock_process.wait.return_value = 0  # Process completes successfully

    # Mock queue with ffmpeg-style output
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = [
        ('stdout', 'frame=  100 fps=25 q=20.0 size=    500kB time=00:00:04.00 bitrate= 1024.0kbits/s'),
        ('stdout', 'frame=  200 fps=25 q=20.0 size=   1000kB time=00:00:08.00 bitrate= 1024.0kbits/s'),
        ('stdout', None),  # EOF for stdout
        ('stderr', None),  # EOF for stderr
        queue.Empty(),  # Simulate timeout
    ]

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")  # Mock thread creation
    mocker.patch("time.sleep")  # Speed up test

    # Call the internal encode method
    encoder._encode_file(mock_video_file, output_path, env)

    # Verify output handling
    assert mock_queue.get.call_count >= 4  # At least 4 queue gets (2 outputs + 2 EOFs)
    assert mock_process.poll.call_count >= 1  # At least one poll check
