"""Unit tests for scene detection functionality"""

import unittest
from pathlib import Path
from drapto.video.scene_detection_helpers import (
    get_candidate_scenes,
    filter_scene_candidates, 
    insert_artificial_boundaries
)
from drapto.video.scene_detection import detect_scenes

class TestSceneDetection(unittest.TestCase):
    def test_filter_scene_candidates(self):
        """Test that scene candidates are properly filtered by minimum gap"""
        candidates = [0.0, 1.0, 1.5, 5.0, 5.2, 10.0]
        min_gap = 2.0
        filtered = filter_scene_candidates(candidates, min_gap)
        self.assertEqual(filtered, [0.0, 5.0, 10.0])
        
    def test_insert_artificial_boundaries(self):
        """Test insertion of boundaries for long gaps"""
        scenes = [0.0, 5.0, 25.0]  # 20 second gap between 5.0 and 25.0
        total_duration = 30.0
        max_length = 10.0
        boundaries = insert_artificial_boundaries(scenes, total_duration)
        # Should insert boundaries at 15.0 to break up the 20s gap
        self.assertEqual(boundaries, [0.0, 5.0, 15.0, 25.0])

if __name__ == '__main__':
    unittest.main()
"""Unit tests for scene detection functionality"""

import unittest
from unittest.mock import patch
from pathlib import Path
from drapto.video.scene_detection import detect_scenes
from drapto.video.scene_detection_helpers import (
    filter_scene_candidates,
    insert_artificial_boundaries
)
from drapto.exceptions import ValidationError

class TestSceneDetection(unittest.TestCase):
    def test_filter_scene_candidates(self):
        """Test filtering of scene candidates"""
        candidates = [0.0, 1.0, 1.5, 5.0, 5.2, 10.0]
        min_gap = 2.0
        filtered = filter_scene_candidates(candidates, min_gap)
        self.assertEqual(filtered, [0.0, 5.0, 10.0])
        
    def test_insert_artificial_boundaries(self):
        """Test insertion of artificial boundaries"""
        scenes = [0.0, 5.0, 25.0]  # 20 second gap between 5.0 and 25.0
        total_duration = 30.0
        boundaries = insert_artificial_boundaries(scenes, total_duration)
        # Should insert boundaries at 15.0 to break up the 20s gap
        self.assertEqual(boundaries, [0.0, 5.0, 15.0, 25.0])

    @patch('drapto.video.scene_detection_helpers.get_candidate_scenes')
    def test_detect_scenes(self, mock_candidates):
        """Test scene detection workflow"""
        mock_candidates.return_value = [0.0, 1.0, 1.5, 5.0, 5.2, 10.0]
        
        test_file = Path("/tmp/test.mkv")
        scenes = detect_scenes(test_file)
        
        # Should return filtered and boundary-inserted scenes
        self.assertEqual(scenes, [0.0, 5.0, 10.0])

if __name__ == '__main__':
    unittest.main()
