import os
import sys

# Ensure the project root is on sys.path. This makes sure that the editable install
# is used correctly and that any third-party package (like rich) installed in the venv is found.
project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
if project_root not in sys.path:
    sys.path.insert(0, project_root)
