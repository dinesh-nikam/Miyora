"""Install the Miyora local TripoSR engine (one-time, needs internet).

    python engines/triposr/setup_engine.py [--engine-root DIR] [--cpu]

Creates <engine-root> (default %LOCALAPPDATA%/Miyora/engines/triposr):
    src/            TripoSR source at a pinned commit (MIT)
    venv/           Python 3.11 environment (via uv)
    hf/, u2net/     cached model weights (TripoSR, DINO tokenizer, rembg u2net)
    run_triposr.py, shim/   Miyora runner + torchmcubes shim (copied from this repo)
    engine.json     manifest read by the desktop app

Idempotent: finished steps are skipped. Uses only the standard library.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

TRIPOSR_REPO = "https://github.com/VAST-AI-Research/TripoSR.git"
TRIPOSR_COMMIT = "107cefdc244c39106fa830359024f6a2f1c78871"
PYTHON_VERSION = "3.11"
TORCH_PACKAGE = "torch==2.7.1"
TORCH_INDEX_CUDA = "https://download.pytorch.org/whl/cu126"
TORCH_INDEX_CPU = "https://download.pytorch.org/whl/cpu"
PACKAGES = [
    "omegaconf==2.3.0",
    "Pillow==10.1.0",
    "einops==0.7.0",
    "transformers==4.35.0",
    "trimesh==4.0.5",
    "rembg[gpu]",
    "huggingface-hub",
    "xatlas==0.0.9",
    "moderngl==5.10.0",
    "PyMCubes",
    "numpy<2",
]
HERE = Path(__file__).resolve().parent


def run(command: list[str], **kwargs) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, check=True, **kwargs)


def default_root() -> Path:
    base = os.environ.get("LOCALAPPDATA") or str(Path.home() / ".local" / "share")
    return Path(base) / "Miyora" / "engines" / "triposr"


def venv_python(root: Path) -> Path:
    return root / "venv" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine-root", type=Path, default=default_root())
    parser.add_argument("--cpu", action="store_true", help="install CPU-only PyTorch")
    args = parser.parse_args()
    root: Path = args.engine_root.resolve()
    root.mkdir(parents=True, exist_ok=True)
    uv = shutil.which("uv")
    git = shutil.which("git")
    if not uv or not git:
        print("uv and git are required on PATH.", file=sys.stderr)
        return 2

    src = root / "src"
    if not (src / ".git").exists():
        run([git, "clone", "--quiet", TRIPOSR_REPO, str(src)])
    run([git, "-C", str(src), "checkout", "--quiet", TRIPOSR_COMMIT])

    python = venv_python(root)
    if not python.exists():
        run([uv, "venv", "--python", PYTHON_VERSION, str(root / "venv")])
    index = TORCH_INDEX_CPU if args.cpu else TORCH_INDEX_CUDA
    run([uv, "pip", "install", "--python", str(python), TORCH_PACKAGE, "--index-url", index])
    run([uv, "pip", "install", "--python", str(python), *PACKAGES])

    shutil.copy2(HERE / "run_triposr.py", root / "run_triposr.py")
    shutil.copytree(HERE / "shim", root / "shim", dirs_exist_ok=True)

    env = {**os.environ, "HF_HOME": str(root / "hf"), "U2NET_HOME": str(root / "u2net"), "HF_HUB_OFFLINE": "0"}
    warm = (
        "import sys; sys.path[:0]=[r'%s', r'%s']\n"
        "from tsr.system import TSR\n"
        "TSR.from_pretrained('stabilityai/TripoSR', config_name='config.yaml', weight_name='model.ckpt')\n"
        "import rembg; rembg.new_session()\n"
        "import torch; print('cuda available:', torch.cuda.is_available())\n"
    ) % (root / "shim", src)
    run([str(python), "-I", "-c", warm], env=env)

    manifest = {
        "engine": "triposr",
        "version": 1,
        "triposrCommit": TRIPOSR_COMMIT,
        # Relative to the engine root, so the folder can be moved or copied.
        "python": python.relative_to(root).as_posix(),
        "runner": "run_triposr.py",
    }
    (root / "engine.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"Engine ready: {root / 'engine.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
