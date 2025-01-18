#!/usr/bin/env python3
import json
import os
import fcntl
import time
import tempfile
from datetime import datetime

def read_json_with_retry(file_path, max_retries=3):
    """Read JSON data with retries and proper locking.
    
    If the file doesn't exist or is empty, initializes it with default data.
    
    Args:
        file_path: Path to the JSON file
        max_retries: Number of times to retry on error
        
    Returns:
        Parsed JSON data
    """
    for attempt in range(max_retries):
        try:
            # First try to read the file
            try:
                with open(file_path, 'r') as f:
                    fcntl.flock(f.fileno(), fcntl.LOCK_EX)
                    try:
                        content = f.read()
                        if not content.strip():
                            raise json.JSONDecodeError('Empty file', '', 0)
                        return json.loads(content)
                    finally:
                        fcntl.flock(f.fileno(), fcntl.LOCK_UN)
            except FileNotFoundError:
                # File doesn't exist, create it with default data
                data = get_default_data(file_path)
                write_json_safely(file_path, data)
                return data
                
        except json.JSONDecodeError as e:
            if attempt == max_retries - 1:
                # On final attempt, reinitialize with default data
                data = get_default_data(file_path)
                write_json_safely(file_path, data)
                return data
            time.sleep(0.1 * (attempt + 1))
        
        except Exception as e:
            if attempt == max_retries - 1:
                raise
            time.sleep(0.1 * (attempt + 1))
    
    return None

def get_default_data(file_path):
    """Get default data structure for a given JSON file type"""
    if file_path.endswith('encoding.json'):
        return {
            'created_at': '',
            'updated_at': '',
            'segments': {},
            'total_attempts': 0,
            'failed_segments': 0
        }
    elif file_path.endswith('segments.json'):
        return {
            'created_at': '',
            'updated_at': '',
            'segments': []
        }
    else:  # progress.json
        return {
            'created_at': '',
            'updated_at': '',
            'current_segment': 0,
            'segments_completed': 0,
            'segments_failed': 0,
            'total_segments': 0
        }

def write_json_safely(file_path, data, create_dirs=True):
    """Write JSON data safely using a temporary file and atomic rename.
    
    Args:
        file_path: Path to the target JSON file
        data: Data to write
        create_dirs: If True, create parent directories if they don't exist
    """
    # Create directory if requested
    dir_path = os.path.dirname(file_path)
    if create_dirs:
        os.makedirs(dir_path, exist_ok=True)
    
    # Create temp file in the same directory
    try:
        with tempfile.NamedTemporaryFile(mode='w', dir=dir_path, prefix='.', suffix='.tmp', delete=False) as temp_file:
            temp_path = temp_file.name
            
            # Set permissions
            os.chmod(temp_path, 0o644)
            
            # Write data with exclusive lock
            fcntl.flock(temp_file.fileno(), fcntl.LOCK_EX)
            try:
                json.dump(data, temp_file, indent=4)
                temp_file.flush()
                os.fsync(temp_file.fileno())
            finally:
                fcntl.flock(temp_file.fileno(), fcntl.LOCK_UN)
        
        # Atomic rename
        os.replace(temp_path, file_path)
        
    except Exception as e:
        # Clean up temp file if something went wrong
        if 'temp_path' in locals() and os.path.exists(temp_path):
            try:
                os.unlink(temp_path)
            except OSError:
                pass
        raise

def get_segment_data(file_path, segment_index):
    """Get segment data from encoding.json"""
    data = read_json_with_retry(file_path)
    segment = data['segments'].get(str(segment_index), {})
    return (
        segment.get('attempts', 0),
        segment.get('last_strategy', '')
    )

def update_segment_status(file_path, index, status, error='', strategy=''):
    """Update segment status in encoding.json and progress.json"""
    # Update encoding.json
    data = read_json_with_retry(file_path)
    
    segment = data['segments'].get(str(index))
    if not segment:
        segment = {
            'status': status,
            'attempts': 0,
            'strategies_tried': [],
            'last_strategy': None,
            'error': None
        }
        data['segments'][str(index)] = segment
    
    segment['status'] = status
    if strategy:
        if 'strategies_tried' not in segment:
            segment['strategies_tried'] = []
        if strategy not in segment['strategies_tried']:
            segment['strategies_tried'].append(strategy)
            segment['attempts'] = len(segment['strategies_tried'])
        segment['last_strategy'] = strategy
    
    if error:
        segment['error'] = error
    else:
        segment['error'] = None

    data['total_attempts'] = data.get('total_attempts', 0) + 1
    if status == 'failed':
        data['failed_segments'] = data.get('failed_segments', 0) + 1

    write_json_safely(file_path, data)

    # Update progress.json
    progress_path = os.path.join(os.path.dirname(file_path), 'progress.json')
    progress = read_json_with_retry(progress_path)

    progress['current_segment'] = int(index)
    if status == 'completed':
        progress['segments_completed'] = progress.get('segments_completed', 0) + 1
    elif status == 'failed':
        progress['segments_failed'] = progress.get('segments_failed', 0) + 1

    write_json_safely(progress_path, progress)

def update_timestamps(dir_path, current_time):
    """Update timestamps in all tracking files"""
    for file in ['segments.json', 'encoding.json', 'progress.json']:
        file_path = os.path.join(dir_path, file)
        try:
            data = read_json_with_retry(file_path)
            
            # Update timestamps
            if not data.get('created_at') or data['created_at'] == '':
                data['created_at'] = current_time
            if not data.get('updated_at') or data['updated_at'] == '':
                data['updated_at'] = current_time
            else:
                data['updated_at'] = current_time
            
            write_json_safely(file_path, data)
        except Exception as e:
            print(f'Error updating {file_path}: {str(e)}')

if __name__ == '__main__':
    import sys
    cmd = sys.argv[1]
    if cmd == 'get_segment_data':
        file_path = sys.argv[2]
        segment_index = sys.argv[3]
        attempts, last_strategy = get_segment_data(file_path, segment_index)
        print(f'{attempts}\n{last_strategy}')
    elif cmd == 'update_segment_status':
        file_path = sys.argv[2]
        index = sys.argv[3]
        status = sys.argv[4]
        error = sys.argv[5] if len(sys.argv) > 5 else ''
        strategy = sys.argv[6] if len(sys.argv) > 6 else ''
        update_segment_status(file_path, index, status, error, strategy)
    elif cmd == 'update_timestamps':
        dir_path = sys.argv[2]
        current_time = sys.argv[3]
        update_timestamps(dir_path, current_time)
