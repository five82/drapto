"""Minimal wrapper for bash encoding scripts."""
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

class Encoder:
    """Minimal wrapper for bash encoding scripts."""
    
    def __init__(self):
        if not hasattr(self, 'script_dir'):
            self.script_dir = Path(__file__).parent / "scripts"
        
        # Ensure script directory exists
        if not self.script_dir.exists():
            raise RuntimeError(f"Script directory not found: {self.script_dir}")
            
        # Main encode script
        self.encode_script = self.script_dir / "encode.sh"
        if not self.encode_script.exists():
            raise RuntimeError(f"Encode script not found: {self.encode_script}")
            
        # Set up environment for shell scripts with color support
        self.env = os.environ.copy()
        
        # Set TERM to support color if needed
        current_term = os.environ.get("TERM", "")
        if not any(x in current_term for x in ["color", "xterm", "vt100"]):
            current_term = "xterm-256color"
            
        self.env.update({
            "SCRIPT_DIR": str(self.script_dir),
            # Force color output
            "FORCE_COLOR": "1",
            "CLICOLOR": "1",
            "CLICOLOR_FORCE": "1",
            "NO_COLOR": "0",  # Disable NO_COLOR if set
            "COLORTERM": "truecolor",  # Indicate full color support
            "TERM": current_term  # Use determined TERM value
        })
        
        # Remove any existing PYTHONPATH to avoid conflicts
        self.env.pop("PYTHONPATH", None)
        
        # Make scripts executable
        for script in self.script_dir.glob("*.sh"):
            script.chmod(0o755)
    
    def _stream_reader(self, stream, queue_obj, stream_name):
        """Read from a stream and put lines into a queue."""
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
        """Clean up process and threads."""
        if process is not None:
            try:
                print("DEBUG: Cleaning up process")
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    print("DEBUG: Force killing process")
                    process.kill()
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        print("DEBUG: Process still not responding after SIGKILL")
                        # One final attempt to wait
                        try:
                            process.wait(timeout=2)
                        except subprocess.TimeoutExpired:
                            print("DEBUG: Final wait attempt failed")
                    except Exception as e:
                        print(f"DEBUG: Error during kill wait: {str(e)}")
            except Exception as e:
                print(f"DEBUG: Error during process cleanup: {str(e)}")
            finally:
                if threads:
                    # Wait for reader threads to finish
                    for thread in threads:
                        thread.join(timeout=1)
                # Final poll to get actual status
                final_status = process.poll()
                print(f"DEBUG: Final process status: {final_status}")
                return final_status

    def _encode_file(self, input_path: Path, output_path: Path, env: dict, is_last_file: bool = True) -> None:
        """Encode a single video file."""
        # Set up environment for this file
        encode_env = env.copy()
        encode_env.update({
            # Set all required paths in environment
            "INPUT_DIR": str(input_path.parent.resolve()),
            "OUTPUT_DIR": str(output_path.parent.resolve()),
            "LOG_DIR": str(Path(encode_env["TEMP_DIR"]) / "logs"),
            "TEMP_DATA_DIR": str(Path(encode_env["TEMP_DIR"]) / "encode_data"),
            "INPUT_FILE": str(input_path.resolve()),
            "OUTPUT_FILE": str(output_path.resolve()),
            # Ensure color output is enabled
            "FORCE_COLOR": "1",
            "CLICOLOR": "1",
            "CLICOLOR_FORCE": "1",
            "NO_COLOR": "0",
            "COLORTERM": "truecolor"
        })

        print(f"DEBUG: Environment variables:")
        print(f"DEBUG: INPUT_DIR: {encode_env['INPUT_DIR']}")
        print(f"DEBUG: OUTPUT_DIR: {encode_env['OUTPUT_DIR']}")
        print(f"DEBUG: LOG_DIR: {encode_env['LOG_DIR']}")
        print(f"DEBUG: TEMP_DATA_DIR: {encode_env['TEMP_DATA_DIR']}")
        print(f"DEBUG: INPUT_FILE: {encode_env['INPUT_FILE']}")
        print(f"DEBUG: OUTPUT_FILE: {encode_env['OUTPUT_FILE']}")

        # Create required directories
        Path(encode_env["LOG_DIR"]).mkdir(parents=True, exist_ok=True)
        Path(encode_env["TEMP_DATA_DIR"]).mkdir(parents=True, exist_ok=True)

        # Clean up any existing state files
        for state_file in Path(encode_env["TEMP_DATA_DIR"]).glob("*.json"):
            state_file.unlink()

        process = None
        output_queue = queue.Queue()
        threads = []

        try:
            print("DEBUG: Starting encode script process")
            # Start encode script with input and output file arguments
            process = subprocess.Popen(
                [str(self.encode_script), str(input_path.resolve()), str(output_path.resolve())],
                env=encode_env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,  # Line buffered
                preexec_fn=os.setsid
            )

            # Start reader threads
            stdout_thread = threading.Thread(
                target=self._stream_reader,
                args=(process.stdout, output_queue, 'stdout')
            )
            stderr_thread = threading.Thread(
                target=self._stream_reader,
                args=(process.stderr, output_queue, 'stderr')
            )
            threads = [stdout_thread, stderr_thread]
            stdout_thread.start()
            stderr_thread.start()

            print("DEBUG: Entering process monitoring loop")
            stdout_done = stderr_done = False

            try:
                while not (stdout_done and stderr_done):
                    try:
                        stream_name, line = output_queue.get(timeout=0.1)
                        if stream_name == 'error':
                            print(f"DEBUG: {line}", file=sys.stderr)
                        elif line is None:
                            if stream_name == 'stdout':
                                stdout_done = True
                            else:
                                stderr_done = True
                        else:
                            if stream_name == 'stdout':
                                print(line)
                                sys.stdout.flush()
                            else:
                                print(line, file=sys.stderr)
                                sys.stderr.flush()
                    except queue.Empty:
                        # Check if process has finished
                        if process.poll() is not None:
                            break

            except KeyboardInterrupt:
                print("DEBUG: KeyboardInterrupt during process monitoring")
                self._cleanup_process(process, threads)
                raise
            except Exception as e:
                print(f"DEBUG: Exception during process monitoring: {str(e)}")
                self._cleanup_process(process, threads)

            # Wait for process completion
            return_code = process.poll()
            if return_code is None:
                print("DEBUG: Process still running, waiting for completion")
                try:
                    return_code = process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    print("DEBUG: Process wait timed out, terminating")
                    return_code = self._cleanup_process(process, threads)
                    if return_code is None:
                        print("DEBUG: Process cleanup failed")
                        return_code = -1  # Force error state
                    elif return_code == 0:
                        print("DEBUG: Process eventually cleaned up successfully")

            # Wait for reader threads to finish
            for thread in threads:
                thread.join(timeout=1)

            print(f"DEBUG: Process completed with return code: {return_code}")
            if return_code != 0:
                # One final check in case the process succeeded after cleanup
                final_status = process.poll()
                if final_status == 0:
                    print("DEBUG: Process eventually succeeded")
                    return_code = 0
                else:
                    raise RuntimeError(f"Encode script failed with return code {return_code}")

        except Exception as e:
            print(f"DEBUG: Exception in _encode_file: {str(e)}")
            # Try cleanup and capture final status
            final_status = self._cleanup_process(process, threads)
            if final_status == 0:
                print("DEBUG: Process succeeded during cleanup")
                return  # Process completed successfully
            raise  # Re-raise the original exception

    def get_files_needing_processing(self, input_path: Path, output_path: Path) -> List[Tuple[Path, Path]]:
        """Returns only the files that need processing with their output paths."""
        print("\nDEBUG: Scanning for files needing processing...")
        work_items = []
        for input_file in input_path.glob("*.mkv"):
            output_file = output_path / input_file.name
            # Always process files unless they exist and have a reasonable size
            needs_processing = not output_file.exists() or output_file.stat().st_size < 1024  # Only skip if file exists and is larger than 1KB
            print(f"DEBUG: File {input_file.name} - exists: {output_file.exists()}, "
                  f"size: {output_file.stat().st_size if output_file.exists() else 0}, "
                  f"needs_processing: {needs_processing}")
            if needs_processing:
                work_items.append((input_file, output_file))
            else:
                print(f"DEBUG: Skipping {input_file.name} - already processed")
        print(f"DEBUG: Found {len(work_items)} files needing processing")
        return work_items

    def encode(self, input_path: Union[str, Path], output_path: Union[str, Path]) -> None:
        """Encode a video file or directory using the configured encoder."""
        try:
            # Process and validate paths
            input_path = Path(input_path).resolve()
            output_path = Path(output_path).resolve()
            
            print(f"\nDEBUG: Starting encode process")
            print(f"DEBUG: Input path: {input_path}")
            print(f"DEBUG: Output path: {output_path}")
            
            if not input_path.exists():
                raise FileNotFoundError(f"Input not found: {input_path}")

            # Set up environment
            env = self.env.copy()  # Use the pre-configured environment with color support
            env["PYTHONUNBUFFERED"] = "1"
            
            # Add drapto package root to PYTHONPATH
            drapto_root = str(Path(__file__).parent.parent.parent)
            env["PYTHONPATH"] = drapto_root + os.pathsep + env.get("PYTHONPATH", "")
            
            # Create temp directory for processing
            temp_dir = tempfile.mkdtemp()
            env["TEMP_DIR"] = temp_dir
            print(f"DEBUG: Created temp directory: {temp_dir}")
            
            try:
                # Handle directory vs file
                if input_path.is_dir():
                    if not output_path.is_dir():
                        output_path.mkdir(parents=True, exist_ok=True)
                    
                    # Get work queue of files needing processing
                    work_queue = self.get_files_needing_processing(input_path, output_path)
                    total_files = len(work_queue)
                    
                    if total_files == 0:
                        print("\nNo files need processing")
                        return
                    
                    print(f"\nFound {total_files} file(s) to process")
                    
                    # Create all required directories once
                    segments_dir = Path(temp_dir) / "segments"
                    encoded_segments_dir = Path(temp_dir) / "encoded_segments"
                    working_dir = Path(temp_dir) / "working"
                    log_dir = Path(temp_dir) / "logs"
                    encode_data_dir = Path(temp_dir) / "encode_data"
                    
                    # Create all directories
                    for dir_path in [segments_dir, encoded_segments_dir, working_dir, log_dir, encode_data_dir]:
                        dir_path.mkdir(parents=True, exist_ok=True)
                        print(f"DEBUG: Created directory: {dir_path}")
                    
                    # Process each file exactly once
                    for i, (input_file, output_file) in enumerate(work_queue):
                        print(f"\nDEBUG: ==================== Starting file {i+1}/{total_files} ====================")
                        print(f"DEBUG: Current file: {input_file.name}")
                        print(f"DEBUG: Next file: {work_queue[i+1][0].name if i < total_files-1 else 'None'}")
                        print(f"DEBUG: Input file exists: {input_file.exists()}")
                        print(f"DEBUG: Output file exists: {output_file.exists()}")
                        if output_file.exists():
                            print(f"DEBUG: Output file size: {output_file.stat().st_size}")
                        
                        # Clean up working directories but preserve logs
                        for dir_path in [segments_dir, encoded_segments_dir, working_dir, encode_data_dir]:
                            if dir_path.exists():
                                print(f"DEBUG: Cleaning up directory: {dir_path}")
                                shutil.rmtree(dir_path)
                            dir_path.mkdir(parents=True, exist_ok=True)
                            print(f"DEBUG: Recreated directory: {dir_path}")
                        
                        print(f"\nProcessing file {i+1} of {total_files}: {input_file.name}")
                        try:
                            print(f"DEBUG: Starting encode_file for {input_file.name}")
                            # Create a new session for each file
                            session_env = env.copy()
                            session_env["SESSION_ID"] = f"encode_session_{i}"
                            self._encode_file(input_file, output_file, session_env, is_last_file=(i == total_files-1))
                            print(f"DEBUG: Finished encode_file for {input_file.name}")
                        except Exception as e:
                            print(f"DEBUG: Error during encode_file for {input_file.name}: {str(e)}")
                            raise
                        
                        # Verify file was processed
                        if output_file.exists():
                            print(f"DEBUG: After processing - Output file exists with size: {output_file.stat().st_size}")
                        else:
                            print(f"DEBUG: After processing - Output file does not exist")
                        
                        print(f"DEBUG: ==================== Completed file {i+1}/{total_files} ====================")
                        print(f"DEBUG: Moving to next file...")
                        
                        # Force flush stdout to ensure we see all debug messages
                        sys.stdout.flush()
                        
                        # Ensure process cleanup
                        print("DEBUG: Waiting for any remaining processes to complete...")
                        time.sleep(2)  # Give processes time to clean up
                        
                        # Kill any lingering processes from the previous session
                        try:
                            subprocess.run(["pkill", "-f", f"SESSION_ID=encode_session_{i}"], 
                                         stderr=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
                        except Exception as e:
                            print(f"DEBUG: Error during process cleanup: {str(e)}")
                        
                        print(f"DEBUG: Ready to process next file")
                else:
                    if output_path.is_dir():
                        output_path = output_path / input_path.name
                    
                    print(f"DEBUG: Processing single file: {input_path}")
                    
                    # Create directories for single file processing
                    segments_dir = Path(temp_dir) / "segments"
                    encoded_segments_dir = Path(temp_dir) / "encoded_segments"
                    working_dir = Path(temp_dir) / "working"
                    log_dir = Path(temp_dir) / "logs"
                    encode_data_dir = Path(temp_dir) / "encode_data"
                    
                    # Create all directories
                    for dir_path in [segments_dir, encoded_segments_dir, working_dir, log_dir, encode_data_dir]:
                        dir_path.mkdir(parents=True, exist_ok=True)
                        print(f"DEBUG: Created directory: {dir_path}")
                    
                    self._encode_file(input_path, output_path, env)
            finally:
                # Clean up temp directory
                print(f"DEBUG: Cleaning up temp directory: {temp_dir}")
                if Path(temp_dir).exists():
                    shutil.rmtree(temp_dir)
        except Exception as e:
            # Clean up temp directory on error
            if 'temp_dir' in locals() and temp_dir and Path(temp_dir).exists():
                print(f"DEBUG: Cleaning up temp directory after error: {temp_dir}")
                shutil.rmtree(temp_dir)
            raise
