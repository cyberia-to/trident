"""Restore path guards; no retained measurement files are changed."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('source_scale_archive', Path(__file__).with_name('archive.py'))
ARCHIVE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ARCHIVE)


class RestorePaths(unittest.TestCase):
    def test_canonical_child_is_confined(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            with patch.object(ARCHIVE, 'ROOT', root):
                self.assertEqual(ARCHIVE.safe('copy/source.tri'), root / 'copy/source.tri')

    def test_lexical_escape_is_rejected(self):
        for name in ('../source.tri', '/source.tri', 'copy/../source.tri', 'copy//source.tri'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'unsafe archive path'):
                ARCHIVE.safe(name)

    def test_parent_symlink_cannot_redirect_restore(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory).resolve()
            root, outside = base / 'root', base / 'outside'
            root.mkdir()
            outside.mkdir()
            (root / 'copy').symlink_to(outside, target_is_directory=True)
            with patch.object(ARCHIVE, 'ROOT', root), self.assertRaisesRegex(ValueError, 'escapes root'):
                ARCHIVE.safe('copy/source.tri')
            self.assertEqual(list(outside.iterdir()), [])


if __name__ == '__main__':
    unittest.main(verbosity=2)
