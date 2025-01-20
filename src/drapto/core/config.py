"""Configuration management for drapto.

This module provides configuration schema validation and management
for all drapto components.
"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, ClassVar, Dict, List, Optional, Type, TypeVar, Union
from multiprocessing import cpu_count

T = TypeVar('T')

@dataclass
class ConfigSchema:
    """Configuration field schema with validation.
    
    Attributes:
        field_name: Name of the configuration field
        field_type: Expected type of the field value
        required: Whether the field is required
        validator: Function to validate the field value
        default: Default value if field is not provided
    """
    field_name: str
    field_type: Type[T]
    required: bool
    validator: Callable[[T], bool]
    default: Optional[T] = None

    def validate(self, value: Any) -> List[str]:
        """Validate a value against this schema.
        
        Args:
            value: Value to validate
            
        Returns:
            List[str]: List of validation error messages, empty if valid
        """
        errors: List[str] = []
        
        if value is None:
            if self.required:
                errors.append(f"Required field '{self.field_name}' is missing")
            return errors
            
        if not isinstance(value, self.field_type):
            errors.append(
                f"Field '{self.field_name}' must be of type {self.field_type.__name__}"
            )
            return errors
            
        try:
            if not self.validator(value):
                errors.append(f"Field '{self.field_name}' failed validation")
        except Exception as e:
            errors.append(f"Validation error for '{self.field_name}': {str(e)}")
            
        return errors

@dataclass
class DraptoConfig:
    """Core drapto configuration with validation.
    
    Attributes:
        temp_dir: Directory for temporary files
        parallel_jobs: Number of parallel encoding jobs
        log_level: Logging level
        hardware_accel: Whether to use hardware acceleration
    """
    temp_dir: Path = field(default=Path("/tmp/drapto"))
    parallel_jobs: int = field(default_factory=lambda: max(1, cpu_count() // 2))
    log_level: str = field(default="INFO")
    hardware_accel: bool = field(default=True)
    
    # Class-level schema definition
    _schema: ClassVar[Dict[str, ConfigSchema]] = {
        'temp_dir': ConfigSchema(
            'temp_dir',
            Path,
            True,
            lambda p: p.parent.exists() or p.parent.is_dir(),
            Path("/tmp/drapto")
        ),
        'parallel_jobs': ConfigSchema(
            'parallel_jobs',
            int,
            True,
            lambda n: 1 <= n <= cpu_count(),
            cpu_count() // 2
        ),
        'log_level': ConfigSchema(
            'log_level',
            str,
            True,
            lambda l: l in ("DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"),
            "INFO"
        ),
        'hardware_accel': ConfigSchema(
            'hardware_accel',
            bool,
            True,
            lambda b: isinstance(b, bool),
            True
        )
    }
    
    def __post_init__(self) -> None:
        """Validate configuration after initialization."""
        errors = self.validate()
        if errors:
            raise ValueError(f"Configuration validation failed: {'; '.join(errors)}")
    
    def validate(self) -> List[str]:
        """Validate the entire configuration.
        
        Returns:
            List[str]: List of validation error messages, empty if valid
        """
        errors: List[str] = []
        
        for field_name, schema in self._schema.items():
            value = getattr(self, field_name)
            field_errors = schema.validate(value)
            errors.extend(field_errors)
            
        return errors

    @classmethod
    def from_dict(cls, config_dict: Dict[str, Any]) -> "DraptoConfig":
        """Create a configuration instance from a dictionary.
        
        Args:
            config_dict: Dictionary containing configuration values
            
        Returns:
            DraptoConfig: Validated configuration instance
            
        Raises:
            ValueError: If configuration is invalid
        """
        # Convert string paths to Path objects
        if 'temp_dir' in config_dict and isinstance(config_dict['temp_dir'], str):
            config_dict['temp_dir'] = Path(config_dict['temp_dir'])
            
        return cls(**config_dict) 