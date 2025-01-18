"""Minimal wrapper for bash encoding scripts."""
import os
import sys
import time
import tempfile
import subprocess
import pty
import select
import fcntl
import termios
import struct
from pathlib import Path
from typing import Optional, Union, List
import errno
import shutil

class Encoder:
    """Minimal wrapper for bash encoding scripts."""
    
    def __init__(self):
        self.script_dir = Path(__file__).parent / "scripts"
        
        # Ensure script directory exists
        if not self.script_dir.exists():
            raise RuntimeError(f"Script directory not found: {self.script_dir}")
            
        # Main encode script
        self.encode_script = self.script_dir / "encode.sh"
        if not self.encode_script.exists():
            raise RuntimeError(f"Encode script not found: {self.encode_script}")
            
        # Set up environment for shell scripts
        self.env = os.environ.copy()
        self.env["SCRIPT_DIR"] = str(self.script_dir)
        # Remove any existing PYTHONPATH to avoid conflicts
        self.env.pop("PYTHONPATH", None)
        
        # Make scripts executable
        for script in self.script_dir.glob("*.sh"):
            script.chmod(0o755)
    
    def _encode_file(self, input_path: Path, output_path: Path, env: dict, is_last_file: bool = True) -> None:
        """Encode a single file."""
        # Create temporary directory for working files
        with tempfile.TemporaryDirectory() as temp_dir:
            # Set up environment
            encode_env = self.env.copy()
            encode_env.update(env)
            encode_env["TEMP_DATA_DIR"] = temp_dir
            encode_env["INPUT_DIR"] = str(input_path.parent)
            encode_env["OUTPUT_DIR"] = str(output_path.parent)
            encode_env["LOG_DIR"] = str(Path(temp_dir) / "logs")

            # Create required directories
            Path(encode_env["LOG_DIR"]).mkdir(parents=True, exist_ok=True)
            Path(temp_dir, "encode_data").mkdir(parents=True, exist_ok=True)

            # Run encode script
            try:
                master_fd, slave_fd = pty.openpty()
                
                # Set terminal size to avoid line wrapping
                term_size = struct.pack('HHHH', 24, 80, 0, 0)
                fcntl.ioctl(slave_fd, termios.TIOCSWINSZ, term_size)
                
                process = subprocess.Popen(
                    [str(self.encode_script), str(input_path), str(output_path)],
                    env=encode_env,
                    stdin=slave_fd,
                    stdout=slave_fd,
                    stderr=slave_fd,
                    preexec_fn=os.setsid
                )

                # Close slave fd as we don't need it
                os.close(slave_fd)
                
                # Set master fd to non-blocking mode
                flags = fcntl.fcntl(master_fd, fcntl.F_GETFL)
                fcntl.fcntl(master_fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)
                
                output_error = None
                try:
                    while True:
                        try:
                            rlist, _, _ = select.select([master_fd], [], [], 0.1)
                            
                            if master_fd in rlist:
                                try:
                                    data = os.read(master_fd, 1024)
                                    if data:
                                        os.write(sys.stdout.fileno(), data)
                                except (OSError, IOError) as e:
                                    if e.errno != errno.EAGAIN:
                                        output_error = e
                                        break
                            
                            # Check if process has finished
                            if process.poll() is not None:
                                # Get remaining output
                                try:
                                    while True:
                                        try:
                                            data = os.read(master_fd, 1024)
                                            if not data:
                                                break
                                            os.write(sys.stdout.fileno(), data)
                                        except (OSError, IOError) as e:
                                            if e.errno != errno.EAGAIN:
                                                output_error = e
                                            break
                                except:
                                    pass
                                break
                        except Exception as e:
                            output_error = e
                            break
                finally:
                    # Restore terminal attributes and close file descriptors
                    try:
                        termios.tcsetattr(master_fd, termios.TCSANOW, old_attr)
                    except:
                        pass
                    
                    try:
                        os.close(master_fd)
                    except:
                        pass
                
                # Check return code
                if process.returncode is None:
                    # Process hasn't finished properly, wait for it
                    process.wait()
                
                if process.returncode != 0:
                    raise RuntimeError(f"Encode script failed with return code {process.returncode}")
                elif output_error and not isinstance(output_error, IOError):
                    # Only raise non-I/O errors that occurred during output handling
                    raise output_error
                
            except Exception as e:
                raise

    def encode(self, input_path: Union[str, Path], output_path: Union[str, Path]) -> None:
        """Encode a video file or directory using the configured encoder."""
        try:
            # Process and validate paths
            input_path = Path(input_path).resolve()
            output_path = Path(output_path).resolve()
            
            if not input_path.exists():
                raise FileNotFoundError(f"Input not found: {input_path}")

            # Set up environment
            env = os.environ.copy()
            env["SCRIPT_DIR"] = str(self.script_dir)
            env["PYTHONUNBUFFERED"] = "1"
            env["FORCE_COLOR"] = "1"
            env["CLICOLOR"] = "1"
            env["CLICOLOR_FORCE"] = "1"
            
            # Add drapto package root to PYTHONPATH
            drapto_root = str(Path(__file__).parent.parent.parent)
            env["PYTHONPATH"] = drapto_root + os.pathsep + env.get("PYTHONPATH", "")
            
            # Preserve existing TERM but ensure it indicates color support
            current_term = os.environ.get("TERM", "")
            if not any(x in current_term for x in ["color", "xterm", "vt100"]):
                current_term = "xterm-256color"
            env["TERM"] = current_term
            
            # Additional color forcing for bash scripts
            env["NO_COLOR"] = "0"  # Disable NO_COLOR if set
            env["COLORTERM"] = "truecolor"  # Indicate full color support
            
            # Create temp directory for processing
            temp_dir = tempfile.mkdtemp()
            env["TEMP_DIR"] = temp_dir
            
            # Create all required subdirectories
            segments_dir = Path(temp_dir) / "segments"
            encoded_segments_dir = Path(temp_dir) / "encoded_segments"
            working_dir = Path(temp_dir) / "working"
            log_dir = Path(temp_dir) / "logs"
            
            # Create all directories
            for dir_path in [segments_dir, encoded_segments_dir, working_dir, log_dir]:
                if dir_path.exists():
                    shutil.rmtree(dir_path)
                dir_path.mkdir(parents=True, exist_ok=True)
            
            # Handle directory vs file
            if input_path.is_dir():
                if not output_path.is_dir():
                    output_path.mkdir(parents=True, exist_ok=True)
                
                # Process each video file in the directory
                files = [f for f in input_path.glob("*.mkv")]
                remaining_files = []
                
                # First, collect files that need processing
                for file in files:
                    out_file = output_path / file.name
                    if not out_file.exists() or out_file.stat().st_size == 0:
                        remaining_files.append(file)
                    else:
                        print(f"\nSkipping {file.name} - already processed")
                
                # Then process only the files that need it
                for i, file in enumerate(remaining_files):
                    out_file = output_path / file.name
                    # Double check the output file doesn't exist right before processing
                    # This handles cases where the file was processed in a previous run
                    # or by another instance
                    if out_file.exists() and out_file.stat().st_size > 0:
                        print(f"\nSkipping {file.name} - already processed")
                        continue
                        
                    self._encode_file(file, out_file, env, is_last_file=(i == len(remaining_files)-1))
                    
                    # Clean up temporary directories after each file
                    for dir_path in [segments_dir, encoded_segments_dir]:
                        if dir_path.exists():
                            shutil.rmtree(dir_path)
                        dir_path.mkdir(parents=True, exist_ok=True)
            else:
                if output_path.is_dir():
                    output_path = output_path / input_path.name
                self._encode_file(input_path, output_path, env)
            
        finally:
            # Clean up temp directory
            if 'temp_dir' in locals() and temp_dir and Path(temp_dir).exists():
                shutil.rmtree(temp_dir)
