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
        
        # Make scripts executable
        for script in self.script_dir.glob("*.sh"):
            script.chmod(0o755)
    
    def _encode_file(self, input_path: Path, output_path: Path, env: dict, is_last_file: bool = True) -> None:
        """Encode a single video file."""
        # Set up environment
        env["INPUT_DIR"] = str(input_path.parent)
        env["OUTPUT_DIR"] = str(output_path.parent)
        env["LOG_DIR"] = str(Path(env["TEMP_DIR"]) / "logs")
        env["INPUT_FILE"] = input_path.name
        env["PRINT_FINAL_SUMMARY"] = "1" if is_last_file else "0"
        
        # Create logs directory
        Path(env["LOG_DIR"]).mkdir(parents=True, exist_ok=True)
        
        # Run encode script with real-time output using pty
        try:
            cmd = [str(self.encode_script)]
            
            # Create pseudo-terminal
            master_fd, slave_fd = pty.openpty()
            
            # Set terminal attributes
            old_attr = termios.tcgetattr(slave_fd)
            new_attr = list(old_attr)
            
            # Enable canonical mode and echo
            new_attr[3] = new_attr[3] | termios.ICANON | termios.ECHO | termios.ISIG
            
            # Enable output processing
            new_attr[1] = new_attr[1] | termios.OPOST | termios.ONLCR
            
            # Apply the new attributes
            termios.tcsetattr(slave_fd, termios.TCSANOW, new_attr)
            
            # Ensure the slave PTY is treated as a TTY
            env["PTY"] = "1"
            
            # Set terminal size to match parent terminal if possible
            try:
                term_size = os.get_terminal_size()
                fcntl.ioctl(slave_fd, termios.TIOCSWINSZ, 
                          struct.pack("HHHH", term_size.lines, term_size.columns, 0, 0))
            except (OSError, AttributeError):
                pass
            
            # Start the process with the PTY as its controlling terminal
            process = subprocess.Popen(
                cmd,
                cwd=str(self.script_dir),
                stdin=slave_fd,
                stdout=slave_fd,
                stderr=slave_fd,
                env=env,
                preexec_fn=os.setsid,  # Create new session
                close_fds=True
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
                files = list(input_path.glob("*.mkv"))
                for i, file in enumerate(files):
                    out_file = output_path / file.name
                    self._encode_file(file, out_file, env, is_last_file=(i == len(files)-1))
                    # Clean up only segments and encoded segments
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
