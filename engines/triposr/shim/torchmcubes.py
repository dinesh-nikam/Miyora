"""Drop-in replacement for the `torchmcubes.marching_cubes` call TripoSR makes.

torchmcubes needs a local C++/CUDA build; PyMCubes ships prebuilt wheels.
torchmcubes returns vertex coordinates in reversed axis order (TripoSR undoes
that with `[..., [2, 1, 0]]`), so this shim reverses PyMCubes' output to match.
Face winding is normalised by the runner (outward normals) after extraction.
"""

import mcubes
import numpy as np
import torch


def marching_cubes(volume: torch.Tensor, threshold: float):
    grid = volume.detach().cpu().numpy().astype(np.float64)
    vertices, faces = mcubes.marching_cubes(grid, threshold)
    vertices = np.ascontiguousarray(vertices[:, ::-1])
    return (
        torch.from_numpy(vertices.astype(np.float32)),
        torch.from_numpy(faces.astype(np.int64)),
    )
