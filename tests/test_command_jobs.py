import unittest
from unittest.mock import patch
from drapto.command_jobs import CommandJob, ProgressCommandJob
from drapto.exceptions import CommandExecutionError

class DummyJob(CommandJob):
    pass  # Use the base behavior for testing

class TestCommandJobs(unittest.TestCase):
    @patch("drapto.command_jobs.run_cmd")
    def test_commandjob_success(self, mock_run_cmd):
        # Arrange
        job = DummyJob(["ls", "-la"])
        # Act
        job.execute()
        # Assert
        mock_run_cmd.assert_called_with(["ls", "-la"])

    @patch("drapto.command_jobs.run_cmd")
    def test_commandjob_failure(self, mock_run_cmd):
        mock_run_cmd.side_effect = Exception("Fake failure")
        job = DummyJob(["ls", "-la"])
        with self.assertRaises(Exception):
            job.execute()

    @patch("drapto.command_jobs.run_cmd_with_progress")
    def test_progress_commandjob_failure(self, mock_run_cmd_with_progress):
        # Simulate a failure return code
        mock_run_cmd_with_progress.return_value = 1
        job = ProgressCommandJob(["echo", "test"])
        with self.assertRaises(Exception):
            job.execute(total_duration=10, log_interval=2)
            
if __name__ == "__main__":
    unittest.main()
