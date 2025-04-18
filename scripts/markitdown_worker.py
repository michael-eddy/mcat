from markitdown import MarkItDown
import sys
import subprocess
import io
import os

# silent
sys.stderr = io.StringIO()
os.environ["PYTHONWARNINGS"] = "ignore"

try:
    import markitdown
except ImportError:
    print("markitdown module not found. Installing...", file=sys.stderr)
    subprocess.check_call([sys.executable, "-m", "pip",
                          "install", "markitdown[all]", "--quiet"])

converter = MarkItDown()
for line in sys.stdin:
    file_path = line.strip()
    try:
        result = converter.convert(file_path)
        sys.stdout.write(result.text_content)
        sys.stdout.write("\0")
        sys.stdout.flush()
    except Exception as e:
        print(f"Error processing {file_path}: {
              e}", file=sys.stderr, flush=True)
