"""Servidor MCP stdio de pesquisa, com catálogo fechado e sem escrita direta no ledger."""

from __future__ import annotations

import argparse
import asyncio
import json
from pathlib import Path

from mcp.server.context import ServerRequestContext
from mcp.server.lowlevel import Server
from mcp.server.stdio import stdio_server
from mcp_types import (
    CallToolRequestParams,
    CallToolResult,
    ListToolsResult,
    PaginatedRequestParams,
    TextContent,
    Tool,
    ToolAnnotations,
)
from pydantic import BaseModel, ValidationError

from logos_client.client import KernelClient, KernelError
from logos_client.protocol import ProtocolError
from logos_client.staging import StagingUploader
from logos_mcp.auditor import AuditConfig, AuditFailure, Auditor
from logos_mcp.schemas import CheckInput, EmptyInput, ObligationInput, ProposeInput, QueryInput
from logos_mcp.service import ResearchService
from logos_provers.farm import Sandbox

CATALOG: dict[str, tuple[type[BaseModel], str]] = {
    "propose_claim": (ProposeInput, "Propõe AIR canônica; apenas o kernel gera O e Pending."),
    "check_claim": (CheckInput, "Recheca LRAT ou roda portfólio; todo aceite exige checker ativo."),
    "find_countermodel": (
        ObligationInput,
        "Busca modelo finito e chama WITNESS/REFUTE; ausência é inconclusiva.",
    ),
    "query_ledger": (
        QueryInput,
        "Consulta cabeça, status por h(Event) ou página de eventos; não autentica assinatura.",
    ),
    "audit_session": (
        EmptyInput,
        "Audita cadeia e certificados com raiz pública fixada pelo operador.",
    ),
    "export_report": (
        EmptyInput,
        "Exporta audit-stream/v1 autenticado e rechecado, exigindo CAS externo.",
    ),
}


def response(value: dict[str, object], *, error: bool = False) -> CallToolResult:
    return CallToolResult(
        content=[TextContent(type="text", text=json.dumps(value, ensure_ascii=False))],
        structured_content=value,
        is_error=error,
    )


def create_server(service: ResearchService) -> Server[None]:
    async def list_tools(
        ctx: ServerRequestContext[None],
        params: PaginatedRequestParams | None,
    ) -> ListToolsResult:
        return ListToolsResult(
            tools=[
                Tool(
                    name=name,
                    description=description,
                    input_schema=model.model_json_schema(),
                    annotations=ToolAnnotations(
                        read_only_hint=name in ("query_ledger", "audit_session", "export_report"),
                        destructive_hint=False,
                        idempotent_hint=name in ("query_ledger", "audit_session", "export_report"),
                        open_world_hint=False,
                    ),
                )
                for name, (model, description) in CATALOG.items()
            ]
        )

    async def call_tool(
        ctx: ServerRequestContext[None],
        params: CallToolRequestParams,
    ) -> CallToolResult:
        arguments = params.arguments if params.arguments is not None else {}
        try:
            if params.name == "propose_claim":
                result = await service.propose(ProposeInput.model_validate(arguments))
            elif params.name == "check_claim":
                result = await service.check(CheckInput.model_validate(arguments))
            elif params.name == "find_countermodel":
                result = await service.countermodel(ObligationInput.model_validate(arguments))
            elif params.name == "query_ledger":
                result = await service.query(QueryInput.model_validate(arguments))
            elif params.name == "audit_session":
                EmptyInput.model_validate(arguments)
                result = await service.audit()
            elif params.name == "export_report":
                EmptyInput.model_validate(arguments)
                result = await service.export()
            else:
                return response({"error": "unknown_tool"}, error=True)
            return response(result)
        except ValidationError:
            return response({"error": "invalid_arguments"}, error=True)
        except KernelError as error:
            return response(
                {
                    "error": "kernel_error",
                    "code": error.code,
                    "submission_may_have_committed": True,
                },
                error=True,
            )
        except AuditFailure:
            return response({"error": "audit_failed", "no_report_returned": True}, error=True)
        except TimeoutError, ConnectionError, OSError, ProtocolError, asyncio.IncompleteReadError:
            return response(
                {
                    "error": "transport_or_deadline",
                    "submission_may_have_committed": True,
                    "retry_automatically": False,
                },
                error=True,
            )
        except ValueError:
            return response({"error": "invalid_context_or_output_limit"}, error=True)

    return Server[None](
        "logos-research", version="5R.1", on_list_tools=list_tools, on_call_tool=call_tool
    )


def configure(args: argparse.Namespace) -> ResearchService:
    with args.vocabulary.open("rb") as public_file:
        vocabulary = public_file.read(1_048_577)
    if len(vocabulary) > 1_048_576:
        raise ValueError("public vocabulary limit")
    return ResearchService(
        KernelClient(args.socket, timeout=args.timeout),
        StagingUploader(args.staging),
        Sandbox(args.encoder, args.solver),
        vocabulary,
        Auditor(
            AuditConfig(
                args.auditor,
                args.cas,
                args.public_key,
                args.cake,
                tuple(args.rust_checker),
            )
        ),
        timeout=args.timeout,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="LOGOS Research MCP, stdio local, sem credenciais")
    for name in (
        "socket",
        "staging",
        "vocabulary",
        "encoder",
        "solver",
        "auditor",
        "cas",
        "cake",
    ):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--public-key", required=True)
    parser.add_argument("--rust-checker", action="append", required=True)
    parser.add_argument("--timeout", type=float, default=30)
    args = parser.parse_args()
    service = configure(args)

    async def run() -> None:
        server = create_server(service)
        async with stdio_server() as (reader, writer):
            await server.run(reader, writer, server.create_initialization_options())

    asyncio.run(run())


if __name__ == "__main__":
    main()
