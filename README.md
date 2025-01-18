# Drapto

Minimal Python wrapper for video encoding bash scripts.

## Installation

```bash
pipx install .

# Or install in development mode
pipx install -e .
```

## Usage

Command line:
```bash
# Encode a single file
drapto input.mkv output.mkv

# Encode a directory of videos
drapto input_dir/ output_dir/
```

Python:
```python
from drapto import Encoder

encoder = Encoder()

# Encode a single file
encoder.encode("input.mkv", "output.mkv")

# Encode a directory
encoder.encode("input_dir", "output_dir")
