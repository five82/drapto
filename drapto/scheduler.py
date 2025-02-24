"""Memory-aware task scheduler for parallel encoding"""

import time
import psutil
import logging
from concurrent.futures import Future
from typing import Dict, Tuple

log = logging.getLogger(__name__)

class MemoryAwareScheduler:
    def __init__(self, base_mem_per_token: int, max_tokens: int, task_stagger_delay: float):
        self.base_mem_per_token = base_mem_per_token
        self.max_tokens = max_tokens
        self.task_stagger_delay = task_stagger_delay
        self.running_tasks: Dict[int, Tuple[Future, int]] = {}

    def current_token_usage(self) -> int:
        """Return the sum of token weights of all running tasks."""
        return sum(token for (_, token) in self.running_tasks.values())

    def can_submit(self, estimated_memory: int) -> bool:
        """
        Determine if a new task can be submitted.
        estimated_memory is already calculated as (token_weight * base_mem_per_token).
        """
        mem = psutil.virtual_memory()
        available_memory = mem.available
        target_available = mem.total * 0.2  # Reserve 20% memory
        current_usage = self.current_token_usage() * self.base_mem_per_token
        
        if available_memory - (current_usage + estimated_memory) > target_available:
            # Check token limits
            if (self.current_token_usage() + (estimated_memory // self.base_mem_per_token)) <= self.max_tokens:
                return True
        return False

    def add_task(self, task_id: int, future: Future, token_weight: int) -> None:
        """Record a submitted task and apply a stagger delay."""
        self.running_tasks[task_id] = (future, token_weight)
        time.sleep(self.task_stagger_delay)

    def update_completed(self) -> None:
        """Remove completed tasks from the running_tasks dict."""
        completed_ids = [tid for tid, (fut, _) in self.running_tasks.items() if fut.done()]
        for tid in completed_ids:
            self.running_tasks.pop(tid)
