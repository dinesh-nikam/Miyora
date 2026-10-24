"""
Tripo 3D API Client
Integrates with the Tripo 3D generation API (v3 with v2 fallback)
for text-to-3D and image-to-3D model generation, task polling, and downloads.

The API key must be set in the TRIPO_API_KEY environment variable.
It is never hard-coded in the source code.
"""

from __future__ import annotations

import io
import json
import logging
import mimetypes
import os
from pathlib import Path
import time
from typing import Any, Callable, Dict, Optional
import urllib.error
import urllib.parse
import urllib.request
import uuid

logger = logging.getLogger("tripo_client")
if not logger.handlers:
    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("[%(asctime)s] [%(levelname)s] [Tripo3D] %(message)s", datefmt="%H:%M:%S"))
    logger.addHandler(handler)
    logger.setLevel(logging.INFO)

# Base endpoints
TRIPO_V3_BASE = "https://openapi.tripo3d.ai/v3"
TRIPO_V2_BASE = "https://api.tripo3d.ai/v2/openapi"

DEFAULT_MODEL_VERSION = "v3.1-20260211"
DEFAULT_POLL_INTERVAL = 3.0
DEFAULT_TIMEOUT = 600.0


class TripoError(Exception):
    """Base exception for Tripo 3D API errors."""
    pass


class TripoAuthError(TripoError):
    """Raised when TRIPO_API_KEY is missing, empty, or invalid."""
    pass


class TripoCreditError(TripoError):
    """Raised when the Tripo account has insufficient credits (code 2010)."""
    pass


class TripoAPIError(TripoError):
    """Raised when the Tripo API returns an error or invalid response."""
    def __init__(self, message: str, status_code: Optional[int] = None, response_body: Optional[str] = None):
        super().__init__(message)
        self.status_code = status_code
        self.response_body = response_body


class TripoGenerationError(TripoError):
    """Raised when 3D model generation fails on the Tripo platform."""
    pass


class TripoTimeoutError(TripoError):
    """Raised when task polling exceeds the allotted timeout limit."""
    pass


class TripoDownloadError(TripoError):
    """Raised when downloading the generated model or assets fails."""
    pass


def get_api_key() -> str:
    """
    Retrieve the Tripo API key from the TRIPO_API_KEY environment variable.
    Raises TripoAuthError if not found or empty.
    """
    key = os.environ.get("TRIPO_API_KEY", "").strip()
    if not key:
        raise TripoAuthError(
            "Missing TRIPO_API_KEY environment variable. "
            "Please obtain an API key from https://platform.tripo3d.ai and set it: "
            "(Windows PowerShell: $env:TRIPO_API_KEY='your_api_key' | Linux/macOS: export TRIPO_API_KEY='your_api_key')."
        )
    return key


def _http_request(
    url: str,
    method: str = "GET",
    data: Optional[bytes] = None,
    headers: Optional[Dict[str, str]] = None,
    timeout: float = 60.0,
) -> Dict[str, Any]:
    """Execute an HTTP request using urllib and return parsed JSON."""
    req_headers = headers or {}
    req = urllib.request.Request(url, data=data, headers=req_headers, method=method)

    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            resp_body = response.read().decode("utf-8")
            status_code = response.getcode()
            try:
                parsed = json.loads(resp_body)
                return parsed
            except json.JSONDecodeError as err:
                raise TripoAPIError(
                    f"Invalid JSON returned from {url}: {err}. Body: {resp_body[:200]}",
                    status_code=status_code,
                    response_body=resp_body,
                ) from err
    except urllib.error.HTTPError as err:
        body = ""
        try:
            body = err.read().decode("utf-8")
        except Exception:
            pass

        parsed_err: Dict[str, Any] = {}
        try:
            parsed_err = json.loads(body)
        except Exception:
            pass

        code = parsed_err.get("code")
        msg = parsed_err.get("message", "")
        suggestion = parsed_err.get("suggestion", "")

        if code == 2010 or "credit" in msg.lower():
            raise TripoCreditError(
                f"Tripo account has insufficient credits (code 2010): {msg}. Suggestion: {suggestion or 'Please purchase or refill credits at https://platform.tripo3d.ai'}"
            ) from err

        if err.code in (401, 403) and ("api key" in msg.lower() or "unauthorized" in msg.lower() or not msg):
            raise TripoAuthError(
                f"Tripo authentication failed (HTTP {err.code}): {msg or body or err.reason}. Verify your TRIPO_API_KEY."
            ) from err

        raise TripoAPIError(
            f"Tripo API error (HTTP {err.code}): {msg or body or err.reason}. {suggestion}".strip(),
            status_code=err.code,
            response_body=body,
        ) from err
    except urllib.error.URLError as err:
        raise TripoAPIError(f"Network error connecting to Tripo API at {url}: {err.reason}") from err


def _upload_file(file_path: Path, api_key: str) -> str:
    """
    Upload a local image file to Tripo v3 file endpoint and return the file_token.
    """
    if not file_path.is_file():
        raise FileNotFoundError(f"Image file does not exist: {file_path}")

    url = f"{TRIPO_V3_BASE}/files"
    boundary = f"----WebKitFormBoundary{uuid.uuid4().hex}"
    filename = file_path.name
    mime_type = mimetypes.guess_type(filename)[0] or "application/octet-stream"

    with open(file_path, "rb") as f:
        file_bytes = f.read()

    body = io.BytesIO()
    body.write(f"--{boundary}\r\n".encode("utf-8"))
    body.write(
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'.encode("utf-8")
    )
    body.write(f"Content-Type: {mime_type}\r\n\r\n".encode("utf-8"))
    body.write(file_bytes)
    body.write(f"\r\n--{boundary}--\r\n".encode("utf-8"))

    content = body.getvalue()
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": f"multipart/form-data; boundary={boundary}",
        "Content-Length": str(len(content)),
    }

    logger.info("Uploading reference image %s (%d bytes) to Tripo...", filename, len(file_bytes))
    resp = _http_request(url, method="POST", data=content, headers=headers)
    if resp.get("code") != 0:
        raise TripoAPIError(f"Failed to upload image to Tripo: {resp.get('message', resp)}")

    file_token = resp.get("data", {}).get("file_token")
    if not file_token:
        raise TripoAPIError(f"Tripo file upload did not return a file_token: {resp}")

    logger.info("Uploaded reference image, received file_token: %s", file_token)
    return file_token


def generate_text_to_3d(prompt: str, **kwargs) -> Dict[str, Any]:
    """
    Initiate a Text-to-3D task on Tripo 3D.

    Args:
        prompt: Detailed generation prompt for the 3D model.
        **kwargs: Additional parameters (e.g. model_version, negative_prompt, face_limit).

    Returns:
        Dict containing task metadata, including 'task_id' and initial 'status'.
    """
    api_key = get_api_key()
    if not prompt or not prompt.strip():
        raise ValueError("Prompt cannot be empty for generate_text_to_3d.")

    clean_prompt = prompt.strip()
    model_version = kwargs.get("model_version", DEFAULT_MODEL_VERSION)
    url = f"{TRIPO_V3_BASE}/generation/text-to-model"

    payload: Dict[str, Any] = {
        "prompt": clean_prompt,
        "model_version": model_version,
    }
    if "negative_prompt" in kwargs:
        payload["negative_prompt"] = kwargs["negative_prompt"]
    if "face_limit" in kwargs:
        payload["face_limit"] = kwargs["face_limit"]

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }

    logger.info("Submitting Text-to-3D task to Tripo API...")
    logger.debug("Prompt: %s", clean_prompt)

    try:
        resp = _http_request(
            url,
            method="POST",
            data=json.dumps(payload).encode("utf-8"),
            headers=headers,
        )
    except TripoAPIError as err:
        # Graceful fallback to v2 endpoint if v3 endpoint is not available
        if err.status_code == 404:
            logger.warning("V3 endpoint 404; falling back to Tripo v2 task endpoint...")
            v2_url = f"{TRIPO_V2_BASE}/task"
            v2_payload = {"type": "text_to_model", "prompt": clean_prompt}
            resp = _http_request(
                v2_url,
                method="POST",
                data=json.dumps(v2_payload).encode("utf-8"),
                headers=headers,
            )
        else:
            raise

    if resp.get("code") != 0:
        raise TripoAPIError(f"Tripo text_to_3d failed: {resp.get('message', resp)}")

    data = resp.get("data", {})
    task_id = data.get("task_id")
    if not task_id:
        raise TripoAPIError(f"Tripo response did not contain task_id: {resp}")

    logger.info("Text-to-3D task created successfully. Task ID: %s", task_id)
    return data


def generate_image_to_3d(image_url_or_file: str, **kwargs) -> Dict[str, Any]:
    """
    Initiate an Image-to-3D task on Tripo 3D.

    Args:
        image_url_or_file: Local file path or public HTTP(S) URL of reference image.
        **kwargs: Additional parameters (e.g. model_version, face_limit).

    Returns:
        Dict containing task metadata, including 'task_id'.
    """
    api_key = get_api_key()
    if not image_url_or_file or not image_url_or_file.strip():
        raise ValueError("Image path or URL cannot be empty for generate_image_to_3d.")

    target = image_url_or_file.strip()
    is_url = target.startswith("http://") or target.startswith("https://")

    if is_url:
        # Download temporarily and upload to get file_token for consistent v3 handling
        logger.info("Downloading remote reference image from %s...", target)
        req = urllib.request.Request(target, headers={"User-Agent": "Miyora-Avatar/1.0"})
        with urllib.request.urlopen(req, timeout=30.0) as img_resp:
            img_bytes = img_resp.read()

        temp_name = Path(urllib.parse.urlparse(target).path).name or "input_image.png"
        boundary = f"----WebKitFormBoundary{uuid.uuid4().hex}"
        body = io.BytesIO()
        body.write(f"--{boundary}\r\n".encode("utf-8"))
        body.write(f'Content-Disposition: form-data; name="file"; filename="{temp_name}"\r\n'.encode("utf-8"))
        body.write(b"Content-Type: image/png\r\n\r\n")
        body.write(img_bytes)
        body.write(f"\r\n--{boundary}--\r\n".encode("utf-8"))
        content = body.getvalue()

        up_headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Content-Length": str(len(content)),
        }
        resp = _http_request(f"{TRIPO_V3_BASE}/files", method="POST", data=content, headers=up_headers)
        if resp.get("code") != 0 or "file_token" not in resp.get("data", {}):
            raise TripoAPIError(f"Failed to upload remote image to Tripo: {resp}")
        file_token = resp["data"]["file_token"]
    else:
        path = Path(target)
        file_token = _upload_file(path, api_key)

    url = f"{TRIPO_V3_BASE}/generation/image-to-model"
    payload = {
        "file_token": file_token,
        "model_version": kwargs.get("model_version", DEFAULT_MODEL_VERSION),
    }
    if "face_limit" in kwargs:
        payload["face_limit"] = kwargs["face_limit"]

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }

    logger.info("Submitting Image-to-3D task to Tripo API...")
    try:
        resp = _http_request(
            url,
            method="POST",
            data=json.dumps(payload).encode("utf-8"),
            headers=headers,
        )
    except TripoAPIError as err:
        if err.status_code == 404:
            logger.warning("V3 endpoint 404; falling back to Tripo v2 task endpoint...")
            v2_url = f"{TRIPO_V2_BASE}/task"
            v2_payload = {
                "type": "image_to_model",
                "file": {"type": "png", "file_token": file_token},
            }
            resp = _http_request(
                v2_url,
                method="POST",
                data=json.dumps(v2_payload).encode("utf-8"),
                headers=headers,
            )
        else:
            raise

    if resp.get("code") != 0:
        raise TripoAPIError(f"Tripo image_to_3d failed: {resp.get('message', resp)}")

    data = resp.get("data", {})
    task_id = data.get("task_id")
    if not task_id:
        raise TripoAPIError(f"Tripo response did not contain task_id: {resp}")

    logger.info("Image-to-3D task created successfully. Task ID: %s", task_id)
    return data


def check_task_status(task_id: str) -> Dict[str, Any]:
    """
    Query the current status and output of a generation task.

    Args:
        task_id: Unique task identifier returned by generation endpoints.

    Returns:
        Dict with status details, progress percentage, and output assets.
    """
    api_key = get_api_key()
    if not task_id or not task_id.strip():
        raise ValueError("task_id cannot be empty.")

    url = f"{TRIPO_V3_BASE}/tasks/{task_id.strip()}"
    headers = {
        "Authorization": f"Bearer {api_key}",
    }

    try:
        resp = _http_request(url, method="GET", headers=headers)
    except TripoAPIError as err:
        if err.status_code == 404:
            # Fallback to v2 status endpoint
            v2_url = f"{TRIPO_V2_BASE}/task/{task_id.strip()}"
            resp = _http_request(v2_url, method="GET", headers=headers)
        else:
            raise

    if resp.get("code") != 0:
        raise TripoAPIError(f"Failed to query task status: {resp.get('message', resp)}")

    data = resp.get("data", {})
    status = data.get("status", "unknown").lower()
    progress = data.get("progress", 0)
    logger.debug("Task %s status: %s (progress: %s%%)", task_id, status, progress)
    return data


def poll_task_until_complete(
    task_id: str,
    timeout: float = DEFAULT_TIMEOUT,
    poll_interval: float = DEFAULT_POLL_INTERVAL,
    progress_callback: Optional[Callable[[Dict[str, Any]], None]] = None,
) -> Dict[str, Any]:
    """
    Poll the task status repeatedly until completed, failed, or timed out.

    Args:
        task_id: Unique task identifier.
        timeout: Maximum seconds to wait before raising TripoTimeoutError.
        poll_interval: Seconds between successive status checks.
        progress_callback: Optional callable invoked with task data on each poll.

    Returns:
        The finalized task data dict with output URLs.
    """
    start_time = time.time()
    last_progress = -1

    while True:
        elapsed = time.time() - start_time
        if elapsed > timeout:
            raise TripoTimeoutError(
                f"Tripo task {task_id} timed out after {elapsed:.1f}s (timeout was {timeout}s)."
            )

        data = check_task_status(task_id)
        status = data.get("status", "").lower()
        progress = data.get("progress", 0)

        if progress != last_progress:
            logger.info("Task %s: %s (%d%%) [elapsed: %.1fs]", task_id, status, progress, elapsed)
            last_progress = progress

        if progress_callback:
            progress_callback(data)

        if status == "success":
            logger.info("Task %s succeeded after %.1fs!", task_id, elapsed)
            return data
        elif status in ("failed", "banned", "cancelled"):
            error_msg = data.get("error") or data.get("message") or f"Task ended with status '{status}'"
            raise TripoGenerationError(f"Tripo 3D generation failed for task {task_id}: {error_msg}")

        time.sleep(poll_interval)


def check_account_balance() -> Dict[str, Any]:
    """
    Query the current API credit balance and usage details from Tripo.

    Returns:
        Dict with 'balance' and 'frozen' credit amounts.
    """
    api_key = get_api_key()
    url = f"{TRIPO_V3_BASE}/account/balance"
    headers = {
        "Authorization": f"Bearer {api_key}",
    }

    resp = _http_request(url, method="GET", headers=headers)
    if resp.get("code") != 0:
        raise TripoAPIError(f"Failed to query account balance: {resp.get('message', resp)}")

    return resp.get("data", {})


def download_model(task_id: str, output_dir: str = "output/models") -> Dict[str, Any]:
    """
    Download the generated 3D model and preview image for a completed task.

    Args:
        task_id: Unique task identifier.
        output_dir: Directory where downloaded model and assets are saved.

    Returns:
        Dict with 'task_id', 'model_path', 'preview_path', and 'status'.
    """
    out_path = Path(output_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    task_data = check_task_status(task_id)
    status = task_data.get("status", "").lower()

    if status != "success":
        raise TripoGenerationError(
            f"Cannot download model for task {task_id} with status '{status}'. Must be 'success'."
        )

    output = task_data.get("output", {})
    # Tripo returns model download URLs under keys: pbr_model, model, model_url, base_model
    model_url = (
        output.get("pbr_model")
        or output.get("model")
        or output.get("model_url")
        or output.get("base_model")
    )

    if not model_url:
        raise TripoDownloadError(f"No 3D model download URL found in task output: {output}")

    # Determine file extension (default .glb)
    url_path = urllib.parse.urlparse(model_url).path
    ext = Path(url_path).suffix or ".glb"
    model_file_path = out_path / f"{task_id}{ext}"

    logger.info("Downloading 3D model from %s -> %s ...", model_url, model_file_path)
    try:
        req = urllib.request.Request(model_url, headers={"User-Agent": "Miyora-Avatar/1.0"})
        with urllib.request.urlopen(req, timeout=120.0) as resp, open(model_file_path, "wb") as out_file:
            bytes_written = 0
            while True:
                chunk = resp.read(65536)
                if not chunk:
                    break
                out_file.write(chunk)
                bytes_written += len(chunk)
    except Exception as err:
        raise TripoDownloadError(f"Failed to download 3D model for task {task_id}: {err}") from err

    logger.info("Downloaded 3D model: %s (%d bytes)", model_file_path, bytes_written)

    # Optional preview image download
    preview_url = (
        output.get("rendered_image")
        or output.get("preview_url")
        or output.get("image")
        or output.get("thumbnail")
    )
    preview_file_path: Optional[Path] = None
    if preview_url:
        p_ext = Path(urllib.parse.urlparse(preview_url).path).suffix or ".png"
        preview_file_path = out_path / f"{task_id}_preview{p_ext}"
        try:
            p_req = urllib.request.Request(preview_url, headers={"User-Agent": "Miyora-Avatar/1.0"})
            with urllib.request.urlopen(p_req, timeout=60.0) as p_resp, open(preview_file_path, "wb") as pf:
                pf.write(p_resp.read())
            logger.info("Downloaded preview image: %s", preview_file_path)
        except Exception as p_err:
            logger.warning("Failed to download preview image for task %s: %p_err", task_id, p_err)
            preview_file_path = None

    return {
        "task_id": task_id,
        "model_path": str(model_file_path.resolve()),
        "preview_path": str(preview_file_path.resolve()) if preview_file_path else None,
        "format": ext.lstrip(".").lower(),
        "file_size": bytes_written,
        "status": "success",
    }
