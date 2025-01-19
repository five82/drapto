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

def test_stream_reader_with_errors(encoder, mocker):
    """Test stream reader error handling."""
    mock_stream = mocker.MagicMock()
    mock_stream.readline.side_effect = [
        IOError("Simulated IO error"),
        "line after error\n",
        ""  # EOF
    ]
    test_queue = queue.Queue()
    
    # Run stream reader
    encoder._stream_reader(mock_stream, test_queue, 'test')
    
    # Check output - should get error message and close
    assert test_queue.get()[0] == 'error'  # Error message
    assert test_queue.get() == ('test', None)  # EOF marker
    assert mock_stream.close.called

def test_stream_reader_with_decode_error(encoder, mocker):
    """Test stream reader handling of decode errors."""
    mock_stream = mocker.MagicMock()
    mock_stream.readline.side_effect = [
        b'\xff\xff\xff\n',  # Invalid UTF-8
        "valid line\n",
        ""  # EOF
    ]
    test_queue = queue.Queue()
    
    # Run stream reader
    encoder._stream_reader(mock_stream, test_queue, 'test')
    
    # Should get error for invalid UTF-8 and continue
    error_msg = test_queue.get()
    assert error_msg[0] == 'error'
    assert 'decode' in str(error_msg[1]).lower()
    assert test_queue.get() == ('test', 'valid line')
    assert test_queue.get() == ('test', None)  # EOF marker

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

def test_encode_file_with_mixed_output(encoder, mock_video_file, temp_dir, mocker):
    """Test handling of mixed stdout/stderr output."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen
    mock_process = mocker.MagicMock()
    mock_process.returncode = None
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    mock_process.poll.side_effect = [None, None, 0]
    mock_process.wait.return_value = 0

    # Mock queue with interleaved stdout/stderr
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = [
        ('stdout', 'Processing started'),
        ('stderr', 'Warning: something minor'),
        ('stdout', 'Progress: 50%'),
        ('stderr', 'Warning: something else'),
        ('stdout', 'Progress: 100%'),
        ('stdout', None),  # stdout EOF
        ('stderr', None),  # stderr EOF
        queue.Empty(),
    ]

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")
    mocker.patch("time.sleep")

    # Call encode_file
    encoder._encode_file(mock_video_file, output_path, env)

    # Verify output handling
    assert mock_queue.get.call_count >= 7  # All messages plus EOFs
    assert mock_process.poll.call_count >= 1

def test_encode_file_with_slow_output(encoder, mock_video_file, temp_dir, mocker):
    """Test handling of slow/delayed output."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen
    mock_process = mocker.MagicMock()
    mock_process.returncode = None
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    mock_process.poll.side_effect = [None] * 5 + [0]  # Several None before completion
    mock_process.wait.return_value = 0

    # Mock queue with delays (Empty exceptions)
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = [
        queue.Empty(),  # Initial delay
        ('stdout', 'Starting...'),
        queue.Empty(),  # Delay
        ('stdout', 'Progress: 50%'),
        queue.Empty(),  # Delay
        ('stdout', 'Progress: 100%'),
        ('stdout', None),  # stdout EOF
        ('stderr', None),  # stderr EOF
        queue.Empty(),
    ]

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")
    mocker.patch("time.sleep")

    # Call encode_file
    encoder._encode_file(mock_video_file, output_path, env)

    # Verify output handling
    assert mock_queue.get.call_count >= 8  # All messages plus EOFs plus Empty cases
    assert mock_process.poll.call_count >= 3  # Multiple polls due to Empty queue

def test_encode_file_process_termination_timeout(encoder, mock_video_file, temp_dir, mocker):
    """Test process termination with timeout handling."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}

    # Mock subprocess.Popen
    mock_process = mocker.MagicMock()
    mock_process.returncode = None
    mock_process.stdout = mocker.MagicMock()
    mock_process.stderr = mocker.MagicMock()
    
    # Set up wait behavior - process resists termination
    wait_results = [
        subprocess.TimeoutExpired("cmd", 2),  # Initial wait timeout
        subprocess.TimeoutExpired("cmd", 2),  # Post-terminate wait timeout
        subprocess.TimeoutExpired("cmd", 2),  # Post-kill wait timeout
        0  # Finally succeeds
    ]
    mock_process.wait = mocker.MagicMock(side_effect=wait_results)
    
    # Process stays alive until killed
    poll_results = [None] * 3 + [0]
    mock_process.poll = mocker.MagicMock(side_effect=poll_results)

    # Mock queue with some output
    mock_queue = mocker.MagicMock()
    mock_queue.get.side_effect = [
        ('stdout', 'Starting...'),
        ('stdout', None),
        ('stderr', None),
        queue.Empty(),
    ]

    mocker.patch("subprocess.Popen", return_value=mock_process)
    mocker.patch("queue.Queue", return_value=mock_queue)
    mocker.patch("threading.Thread")
    mocker.patch("time.sleep")

    # Call encode_file - should handle the timeouts
    encoder._encode_file(mock_video_file, output_path, env)

    # Verify cleanup sequence
    assert mock_process.terminate.call_count >= 1  # SIGTERM attempted
    assert mock_process.kill.call_count >= 1  # SIGKILL used
    assert mock_process.wait.call_count >= 3  # Multiple waits attempted
