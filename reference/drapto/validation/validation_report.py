"""Validation report formatting and presentation

Responsibilities:
- Format validation results into a readable report
- Present validation status with appropriate styling
- Handle error reporting and summary statistics
"""

import logging
from typing import List

from ..formatting import print_check, print_error, print_header
from ..exceptions import ValidationError

logger = logging.getLogger(__name__)

def format_validation_report(validation_report: List[str], has_errors: bool = False) -> None:
    """Format and print the validation report with appropriate styling"""
    if validation_report:
        print_header("Validation Report")
        for entry in validation_report:
            if entry.startswith("ERROR"):
                print_error(entry[7:])
            else:
                print_check(entry)

    if any(entry.startswith("ERROR") for entry in validation_report) or has_errors:
        raise ValidationError(
            "Output validation failed with the above issues",
            module="validation"
        )
    
    print_check("Output validation successful")
