"""Core encoding functionality."""
import os
import sys
import time
import tempfile
import subprocess
from pathlib import Path
from typing import Optional, Union, List, Tuple
import errno
import shutil
import threading
import queue

from .config import Settings
from .utils.paths import (
    ensure_directory,
    normalize_path,
    get_temp_path,
    get_relative_path
)
from .monitoring.paths import path_monitor


class Encoder:
    """Core encoding functionality."""
    
    def __init__(self, settings: Optional[Settings] = None):
        """Initialize encoder with optional settings.
        
        Args:
            settings: Optional settings object. If not provided, settings will be loaded
                    from environment variables.
        
        Raises:
            RuntimeError: If encode script is not found
        """
        # Initialize settings
        self.settings = settings or Settings.from_environment()
        
        print(f"Debug: Using script directory: {self.settings.paths.script_dir}")
        
        # Main encode script
        self.encode_script = self.settings.paths.script_dir / "encode.sh"
        if not self.encode_script.exists():
            raise RuntimeError(f"Encode script not found: {self.encode_script}")
            
        # Make scripts executable and track them
        for script in self.settings.paths.script_dir.glob("*.sh"):
            script.chmod(0o755)
            path_monitor.track_path(script)
            path_monitor.record_access(script)
        
        print(f"Debug: Using temp directory: {self.settings.paths.temp_dir}")
        
        # Ensure temp directories exist
        ensure_directory(self.settings.paths.temp_dir)
        ensure_directory(self.settings.paths.log_dir)
        ensure_directory(self.settings.paths.temp_data_dir)
        ensure_directory(self.settings.paths.segments_dir)
        ensure_directory(self.settings.paths.encoded_segments_dir)
        ensure_directory(self.settings.paths.working_dir)
        
        # Track temp directories
        path_monitor.track_path(self.settings.paths.temp_dir)
        path_monitor.track_path(self.settings.paths.log_dir)
        path_monitor.track_path(self.settings.paths.temp_data_dir)
        path_monitor.track_path(self.settings.paths.segments_dir)
        path_monitor.track_path(self.settings.paths.encoded_segments_dir)
        path_monitor.track_path(self.settings.paths.working_dir)
    
    def _stream_reader(self, stream, queue_obj, stream_name):
        """Read from a stream and put lines into a queue.
        
        Args:
            stream: The stream to read from
            queue_obj: Queue to put lines into
            stream_name: Name of the stream for identification
        """
        try:
            for line in iter(stream.readline, ''):
                if line:
                    try:
                        # Handle both string and bytes input
                        if isinstance(line, bytes):
                            line = line.decode()
                        queue_obj.put((stream_name, line.rstrip()))
                    except (UnicodeDecodeError, UnicodeError) as e:
                        queue_obj.put(('error', f"Decode error in {stream_name}: {str(e)}"))
        except Exception as e:
            queue_obj.put(('error', f"Error reading from {stream_name}: {str(e)}"))
        finally:
            stream.close()
            queue_obj.put((stream_name, None))  # Signal EOF
    
    def _cleanup_process(self, process, threads=None):
        """Clean up a process and its threads.
        
        Args:
            process: The process to clean up
            threads: Optional list of threads to clean up
        """
        if process:
            try:
                process.terminate()
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
            except Exception:
                pass
        
        if threads:
            for thread in threads:
                try:
                    thread.join(timeout=1)
                except Exception:
                    pass
    
    def get_files_needing_processing(self, input_path: Path, output_path: Path) -> List[Tuple[Path, Path]]:
        """Get list of files that need to be encoded.
        
        Args:
            input_path: Input path (file or directory)
            output_path: Output path (file or directory)
        
        Returns:
            List of (input_path, output_path) tuples
        
        Raises:
            ValueError: If paths are invalid
        """
        # Convert to absolute paths
        input_path = normalize_path(input_path).resolve()
        output_path = normalize_path(output_path).resolve()
        
        # Track paths
        path_monitor.track_path(input_path)
        path_monitor.track_path(output_path)
        
        if input_path.is_file():
            # Single file mode
            if not any(input_path.name.endswith(f".{ext}") for ext in self.settings.paths.input_extensions):
                raise ValueError(f"Input file must have one of these extensions: {', '.join(self.settings.paths.input_extensions)}")
            return [(input_path, output_path)]
        
        # Directory mode
        if not input_path.is_dir():
            raise ValueError(f"Input directory not found: {input_path}")
        if not output_path.is_dir() and not output_path.parent.exists():
            raise ValueError(f"Output directory does not exist: {output_path.parent}")
        
        # Create output directory if needed
        if not output_path.exists():
            ensure_directory(output_path)
        
        # Find all input files
        result = []
        for ext in self.settings.paths.input_extensions:
            for in_file in input_path.rglob(f"*.{ext}"):
                try:
                    # Get relative path from input directory
                    rel_path = get_relative_path(in_file, input_path)
                    out_file = output_path / rel_path.with_suffix(".mkv")
                    
                    # Create output directory if needed
                    if not out_file.parent.exists():
                        ensure_directory(out_file.parent)
                    
                    result.append((in_file, out_file))
                except Exception as e:
                    path_monitor.record_error(in_file, str(e))
        
        if not result:
            raise ValueError(f"No files found to process in {input_path}")
        
        return result
    
    def encode(self, input_path: Union[str, Path], output_path: Union[str, Path]) -> None:
        """Encode video files.
        
        Args:
            input_path: Input path (file or directory)
            output_path: Output path (file or directory)
        
        Raises:
            ValueError: If paths are invalid
            RuntimeError: If encoding fails
        """
        # Get files to process
        files = self.get_files_needing_processing(input_path, output_path)
        
        # Process files
        for i, (in_file, out_file) in enumerate(files):
            is_last = i == len(files) - 1
            self._encode_file(in_file, out_file, is_last)
    
    def _encode_file(self, input_path: Path, output_path: Path, is_last_file: bool = True) -> None:
        """Encode a single file.
        
        Args:
            input_path: Path to input file
            output_path: Path to output file
            is_last_file: Whether this is the last file to encode
        
        Raises:
            RuntimeError: If encoding fails
        """
        # Track input and output paths
        path_monitor.track_path(input_path)
        path_monitor.track_path(output_path)
        path_monitor.record_access(input_path)
        
        print(f"Debug: Processing file: {input_path} -> {output_path}")
        
        # Create unique working directory for this file
        work_dir = get_temp_path(
            self.settings.paths.working_dir,
            prefix=f"encode_{input_path.stem}_"
        )
        ensure_directory(work_dir)
        path_monitor.track_path(work_dir)
        
        print(f"Debug: Using working directory: {work_dir}")
        
        try:
            # Prepare environment
            env = os.environ.copy()
            env_vars = {
                "DRAPTO_TEMP_DIR": str(self.settings.paths.temp_dir),
                "DRAPTO_LOG_DIR": str(self.settings.paths.log_dir),
                "DRAPTO_TEMP_DATA_DIR": str(self.settings.paths.temp_data_dir),
                "DRAPTO_SEGMENTS_DIR": str(self.settings.paths.segments_dir),
                "DRAPTO_ENCODED_SEGMENTS_DIR": str(self.settings.paths.encoded_segments_dir),
                "DRAPTO_WORKING_DIR": str(work_dir),
                "DRAPTO_INPUT_FILE": str(input_path),
                "DRAPTO_OUTPUT_FILE": str(output_path),
                "DRAPTO_IS_LAST_FILE": "1" if is_last_file else "0",
                "DRAPTO_SCRIPT_DIR": str(self.settings.paths.script_dir)
            }
            env.update(env_vars)
            
            print("Debug: Environment variables:")
            for key, value in env_vars.items():
                print(f"  {key}={value}")
            
            # Start encoding process
            process = subprocess.Popen(
                [str(self.encode_script)],
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                bufsize=1,
                universal_newlines=True
            )
            
            # Set up output handling
            output_queue = queue.Queue()
            stdout_thread = threading.Thread(
                target=self._stream_reader,
                args=(process.stdout, output_queue, 'stdout')
            )
            stderr_thread = threading.Thread(
                target=self._stream_reader,
                args=(process.stderr, output_queue, 'stderr')
            )
            threads = [stdout_thread, stderr_thread]
            
            # Start output threads
            for thread in threads:
                thread.daemon = True
                thread.start()
            
            # Process output
            error_lines = []
            eof_count = 0
            while eof_count < len(threads):
                try:
                    stream_name, line = output_queue.get(timeout=0.1)
                    if line is None:
                        eof_count += 1
                    elif stream_name == 'error':
                        error_lines.append(line)
                    elif stream_name == 'stderr':
                        print(line, file=sys.stderr)
                    else:
                        print(line)
                except queue.Empty:
                    # Check if process is still alive
                    if process.poll() is not None:
                        break
            
            # Wait for process to finish
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self._cleanup_process(process, threads)
                raise RuntimeError("Encoding process timed out")
            
            # Check for errors
            if process.returncode != 0:
                if error_lines:
                    raise RuntimeError("\n".join(error_lines))
                raise RuntimeError(f"Encoding failed with return code {process.returncode}")
            
            # Record successful output
            path_monitor.record_access(output_path)
            
        except Exception as e:
            path_monitor.record_error(input_path, str(e))
            raise
        finally:
            # Clean up working directory
            try:
                shutil.rmtree(work_dir)
            except Exception as e:
                path_monitor.record_error(work_dir, f"Failed to clean up: {e}")
