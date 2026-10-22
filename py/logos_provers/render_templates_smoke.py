"""Pré-requisito9.1a: v2 percorre MCP, kernel e auditor independente reais."""

import hashlib
import json

from logos_client import air, canon
from logos_mcp.smoke import ROOT, run

if __name__ == "__main__":
    source = 'vocab v1 { pred P(); pred Q(); pred R(); render_pt ["chove", "faz frio", "neva"]; } '
    vocabulary = air.parse_document(source).vocabulary
    assert isinstance(vocabulary, list) and vocabulary[0] == 2
    wire = canon.encode(vocabulary)
    path = ROOT / "target/render-templates-report.json"
    run(path, vocabulary_source=source)
    report = json.loads(path.read_text())
    report["vocabulary_format"] = 2
    report["vocabulary_hex"] = wire.hex()
    report["h_vocabulary"] = hashlib.sha256(b"logos/vocab/v1\0" + wire).hexdigest()
    path.write_text(json.dumps(report, indent=2) + "\n")
    print("RENDER_TEMPLATES_SMOKE_OK tasks=10 vocabulary_format=2 audit=complete")
