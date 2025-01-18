"""Unit tests for cli.py."""
import pytest
from click.testing import CliRunner
from drapto.cli import main

def test_cli_missing_input(tmp_path):
    """Test CLI with missing input file."""
    runner = CliRunner()
    result = runner.invoke(main, [str(tmp_path / "nonexistent.mp4"), "output.mp4"])
    assert result.exit_code == 2
    assert "does not exist" in result.output

def test_cli_with_valid_paths(mock_video_file, temp_dir, mocker):
    """Test CLI with valid input and output paths."""
    # Mock the Encoder to avoid actual encoding
    mock_encode = mocker.patch("drapto.cli.Encoder")
    mock_encode.return_value.encode.return_value = None
    
    runner = CliRunner()
    output_path = temp_dir / "output.mp4"
    
    result = runner.invoke(main, [str(mock_video_file), str(output_path)])
    
    assert result.exit_code == 0
    mock_encode.return_value.encode.assert_called_once_with(
        str(mock_video_file),
        str(output_path)
    )
