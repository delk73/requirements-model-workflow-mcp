#!/usr/bin/env python3

import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("check_text_files.py")
SPEC = importlib.util.spec_from_file_location("check_text_files", MODULE_PATH)
CHECKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(CHECKER)


class CheckBytesTests(unittest.TestCase):
    def test_valid_lf_file(self):
        self.assertEqual(CHECKER.check_bytes(b"line one\nline two\n"), [])

    def test_missing_final_newline(self):
        self.assertIn("does not end with exactly one final LF", CHECKER.check_bytes(b"line"))

    def test_crlf_content(self):
        self.assertIn("contains carriage-return bytes", CHECKER.check_bytes(b"line\r\n"))

    def test_lone_carriage_return_content(self):
        self.assertIn(
            "contains carriage-return bytes",
            CHECKER.check_bytes(b"line one\rline two\n"),
        )

    def test_multiple_trailing_newlines(self):
        self.assertIn("has more than one trailing LF", CHECKER.check_bytes(b"line\n\n"))

    def test_empty_file(self):
        self.assertEqual(CHECKER.check_bytes(b""), [])

    def test_ignored_extension(self):
        self.assertFalse(CHECKER.is_policy_path(Path("image.bin")))

    def test_editorconfig_is_classified_by_filename(self):
        self.assertTrue(CHECKER.is_policy_path(Path(".editorconfig")))

    def test_extensionless_tracked_text_files_are_classified_by_filename(self):
        self.assertTrue(CHECKER.is_policy_path(Path("Cargo.lock")))
        self.assertTrue(CHECKER.is_policy_path(Path("LICENSE")))


if __name__ == "__main__":
    unittest.main()
