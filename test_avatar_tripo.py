"""
Test Suite for 3D Avatar Generation via Tripo 3D API & Post-Processing
Tests prompt sanitization, error handling for missing TRIPO_API_KEY,
task status checking, download logic, and model post-processing.
"""

import os
from pathlib import Path
import unittest
from unittest.mock import MagicMock, patch

from anime_character_prompt import (
    BANNED_FRANCHISE_NAMES,
    build_anime_prompt,
    sanitize_copyright,
    validate_prompt_originality,
)
from blender_postprocess import _compute_bounds, find_blender_binary, postprocess_model
from tripo_client import (
    TripoAPIError,
    TripoAuthError,
    TripoDownloadError,
    TripoError,
    TripoGenerationError,
    TripoTimeoutError,
    check_task_status,
    download_model,
    generate_image_to_3d,
    generate_text_to_3d,
    get_api_key,
    poll_task_until_complete,
)
from avatar_workflow import run_avatar_generation


class TestAnimeCharacterPrompt(unittest.TestCase):
    def test_example_pink_hair_girl(self):
        user_input = "anime girl with pink hair"
        result = build_anime_prompt(user_input)
        self.assertIn("Original full-body 3D anime-style girl character", result)
        self.assertIn("long pink hair", result)
        self.assertIn("large expressive eyes", result)
        self.assertIn("stylish modern outfit", result)
        self.assertIn("T-pose", result)
        self.assertIn("clean topology", result)
        self.assertIn("game-ready 3D model", result)

    def test_copyright_sanitization_nezuko(self):
        user_input = "Nezuko in modern clothes"
        result = build_anime_prompt(user_input)
        self.assertNotIn("nezuko", result.lower())
        self.assertTrue(validate_prompt_originality(result))
        self.assertIn("Original", result)

    def test_all_banned_franchise_names_sanitized(self):
        for name in ["Naruto", "Goku", "Pikachu", "Luffy", "Gojo", "Tanjiro", "Sailor Moon"]:
            inp = f"make a 3D model of {name} looking cool"
            res = build_anime_prompt(inp)
            self.assertNotIn(name.lower(), res.lower())
            self.assertTrue(validate_prompt_originality(res))

    def test_user_detailed_prompt_preserved(self):
        prompt = (
            "Original 3D anime-style girl character, long black-to-pink gradient hair, pink eyes, "
            "Japanese-inspired modern outfit, bamboo-inspired hair accessory, gentle expression, "
            "full body, T-pose, clean topology, game-ready."
        )
        res = build_anime_prompt(prompt)
        self.assertTrue(res.startswith("Original"))
        self.assertIn("black-to-pink gradient hair", res)
        self.assertIn("bamboo-inspired hair accessory", res)
        self.assertTrue(validate_prompt_originality(res))


class TestTripoClientErrorHandling(unittest.TestCase):
    def test_missing_api_key_raises_auth_error(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(TripoAuthError) as ctx:
                get_api_key()
            self.assertIn("Missing TRIPO_API_KEY", str(ctx.exception))

            with self.assertRaises(TripoAuthError):
                generate_text_to_3d("test prompt")

            with self.assertRaises(TripoAuthError):
                generate_image_to_3d("test.png")

            with self.assertRaises(TripoAuthError):
                check_task_status("task-123")

    def test_empty_prompt_raises_value_error(self):
        with patch.dict(os.environ, {"TRIPO_API_KEY": "dummy_key"}):
            with self.assertRaises(ValueError):
                generate_text_to_3d("")

    def test_task_failure_raises_generation_error(self):
        with patch.dict(os.environ, {"TRIPO_API_KEY": "dummy_key"}):
            with patch("tripo_client.check_task_status") as mock_status:
                mock_status.return_value = {
                    "task_id": "task-fail",
                    "status": "failed",
                    "error": "GPU memory allocation exceeded",
                }
                with self.assertRaises(TripoGenerationError) as ctx:
                    poll_task_until_complete("task-fail", timeout=5, poll_interval=0.1)
                self.assertIn("GPU memory allocation", str(ctx.exception))

    def test_task_timeout_raises_timeout_error(self):
        with patch.dict(os.environ, {"TRIPO_API_KEY": "dummy_key"}):
            with patch("tripo_client.check_task_status") as mock_status:
                mock_status.return_value = {
                    "task_id": "task-running",
                    "status": "running",
                    "progress": 42,
                }
                with self.assertRaises(TripoTimeoutError):
                    poll_task_until_complete("task-running", timeout=0.2, poll_interval=0.1)

    def test_download_model_missing_url_raises_download_error(self):
        with patch.dict(os.environ, {"TRIPO_API_KEY": "dummy_key"}):
            with patch("tripo_client.check_task_status") as mock_status:
                mock_status.return_value = {
                    "task_id": "task-ok",
                    "status": "success",
                    "output": {},
                }
                with self.assertRaises(TripoDownloadError):
                    download_model("task-ok", output_dir="output/models")


class TestAvatarWorkflow(unittest.TestCase):
    def test_workflow_missing_key_graceful_result(self):
        with patch.dict(os.environ, {}, clear=True):
            res = run_avatar_generation(prompt="anime girl with pink hair")
            self.assertEqual(res["status"], "failed")
            self.assertIn("Missing TRIPO_API_KEY", res["error"])

    def test_workflow_end_to_end_mocked(self):
        with patch.dict(os.environ, {"TRIPO_API_KEY": "dummy_key"}):
            with patch("avatar_workflow.generate_text_to_3d") as mock_gen, \
                 patch("avatar_workflow.poll_task_until_complete") as mock_poll, \
                 patch("avatar_workflow.download_model") as mock_dl, \
                 patch("avatar_workflow.postprocess_model") as mock_post:

                mock_gen.return_value = {"task_id": "task_mock_777"}
                mock_poll.return_value = {"status": "success"}
                mock_dl.return_value = {
                    "task_id": "task_mock_777",
                    "model_path": str(Path("output/models/task_mock_777.glb").resolve()),
                    "preview_path": str(Path("output/models/task_mock_777_preview.png").resolve()),
                }
                mock_post.return_value = {
                    "status": "success",
                    "preview_path": str(Path("output/models/task_mock_777_rendered.png").resolve()),
                }

                res = run_avatar_generation(prompt="anime girl with pink hair")
                self.assertEqual(res["status"], "success")
                self.assertEqual(res["task_id"], "task_mock_777")
                self.assertIsNotNone(res["model_path"])
                self.assertIsNotNone(res["preview_image_path"])
                self.assertIsNone(res["error"])


if __name__ == "__main__":
    unittest.main()
