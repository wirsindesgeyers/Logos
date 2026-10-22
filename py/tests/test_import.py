"""Smoke test: os seis pacotes Python do projeto importam."""

import importlib

import pytest

PACKAGES = [
    "logos_client",
    "logos_provers",
    "logos_mcp",
    "logos_notebook",
    "logos_agent",
    "logos_eval",
]


@pytest.mark.parametrize("name", PACKAGES)
def test_package_imports(name: str) -> None:
    module = importlib.import_module(name)
    assert module.__doc__
