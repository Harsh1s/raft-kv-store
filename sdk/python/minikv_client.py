"""Notebook-first Python client for minikv.

This client is intentionally lightweight and focuses on the endpoints needed for
analytics and ML workflows:
- time-series write/query
- vector upsert/query
- range and batch access
- backup/restore and metrics access
- change streams via Server-Sent Events
"""

from __future__ import annotations

import json
import importlib
from dataclasses import dataclass
from typing import Any, Dict, Generator, Iterable, List, Optional

requests = None


@dataclass
class MinikvConfig:
    base_url: str = "http://localhost:8080"
    api_key: Optional[str] = None
    timeout_seconds: int = 15


class MinikvClient:
    def __init__(self, config: Optional[MinikvConfig] = None) -> None:
        global requests
        if requests is None:
            try:
                requests = importlib.import_module("requests")
            except ModuleNotFoundError as exc:
                raise RuntimeError(
                    "requests is required. Install dependencies with: "
                    "pip install -r sdk/python/requirements.txt"
                ) from exc

        if requests is None:
            raise RuntimeError(
                "requests is required. Install dependencies with: "
                "pip install -r sdk/python/requirements.txt"
            )
        self.config = config or MinikvConfig()
        self._session = requests.Session()
        self._session.headers.update({"Content-Type": "application/json"})
        if self.config.api_key:
            self._session.headers.update({"Authorization": f"Bearer {self.config.api_key}"})

    def _url(self, path: str) -> str:
        return f"{self.config.base_url.rstrip('/')}/{path.lstrip('/')}"

    def _json(self, method: str, path: str, **kwargs: Any) -> Dict[str, Any]:
        resp = self._session.request(
            method=method,
            url=self._url(path),
            timeout=self.config.timeout_seconds,
            **kwargs,
