"""Unit tests for core.py."""
import os
import pytest
from pathlib import Path
from drapto.core import Encoder

def test_encoder_initialization(encoder):
    """Test that Encoder initializes correctly."""
    assert encoder.script_dir.exists()
    assert encoder.encode_script.exists()
    assert encoder.encode_script.is_file()
    assert os.access(encoder.encode_script, os.X_OK)

def test_encoder_with_invalid_script_dir(monkeypatch, tmp_path):
    """Test Encoder initialization with invalid script directory."""
    def mock_parent(self):
        return tmp_path
    monkeypatch.setattr(Path, "parent", property(mock_parent))
    with pytest.raises(RuntimeError, match="Script directory not found"):
        Encoder()

def test_encode_file_environment_setup(encoder, mock_video_file, temp_dir, mocker):
    """Test environment setup for encoding."""
    output_path = temp_dir / "output.mp4"
    env = {"TEMP_DIR": str(temp_dir)}
    
    # Mock the subprocess.Popen to avoid actual execution
    mock_process = mocker.MagicMock()
    mock_process.returncode = 0
    mock_process.poll.return_value = 0
    mock_popen = mocker.patch("subprocess.Popen", return_value=mock_process)
    
    # Mock PTY operations
    mocker.patch("pty.openpty", return_value=(0, 1))
    mocker.patch("termios.tcgetattr", return_value=[0] * 7)  # Mock terminal attributes
    mocker.patch("termios.tcsetattr")
    mocker.patch("os.close")
    mocker.patch("fcntl.fcntl")
    mocker.patch("os.get_terminal_size", side_effect=OSError)  # Simulate no terminal
    mocker.patch("select.select", return_value=([0], [], []))  # Simulate ready to read
    mocker.patch("os.read", side_effect=["some output".encode(), b""])  # Simulate some output then EOF
    mocker.patch("os.write")  # Mock writing to stdout
    
    # Call the internal encode method
    encoder._encode_file(mock_video_file, output_path, env)
    
    # Verify environment setup
    call_env = mock_popen.call_args[1]["env"]
    assert call_env["INPUT_DIR"] == str(mock_video_file.parent)
    assert call_env["OUTPUT_DIR"] == str(output_path.parent)
    assert call_env["LOG_DIR"] == str(Path(env["TEMP_DIR"]) / "logs")
    assert call_env["INPUT_FILE"] == mock_video_file.name
    assert call_env["PTY"] == "1"  # Verify PTY environment variable is set
