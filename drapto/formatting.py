"""Rich-based console formatting utilities"""

from rich.console import Console
from rich.text import Text
from rich.panel import Panel

console = Console()

def print_check(message: str) -> None:
    """Print a checkmark message in bold green."""
    text = Text("✓ ", style="bold green") + Text(message, style="bold")
    console.print(text)

def print_warning(message: str) -> None:
    """Print a warning message in bold yellow."""
    text = Text("⚠ ", style="bold yellow") + Text(message, style="bold")
    console.print(text)

def print_error(message: str) -> None:
    """Print an error message in bold red."""
    text = Text("✗ ", style="bold red") + Text(message, style="bold")
    console.print(text)

def print_success(message: str) -> None:
    """Print a success message in plain green."""
    text = Text("✓ ", style="green") + Text(message, style="green")
    console.print(text)

def print_header(title: str, width: int = 80) -> None:
    """Print a decorative header."""
    separator = Text("=" * width, style="bold blue")
    padding = (width - len(title)) // 2
    title_line = " " * padding + title
    console.print(separator)
    console.print(title_line, style="bold blue")
    console.print(separator)

def print_separator() -> None:
    """Print a separator line."""
    console.print("-" * 40, style="blue")

def print_info(message: str) -> None:
    """Print an informational message in a subtle style."""
    text = Text("ℹ ", style="bold blue") + Text(message, style="blue")
    console.print(text)
