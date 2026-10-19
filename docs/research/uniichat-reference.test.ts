// Auditoria opcional: executa funções do upstream fixado, sem provider real.
// LOGOS_UNIICHAT_REFERENCE=/tmp/logos-uniichat-audit bun test --isolate <este arquivo>
// Não é implementação de memória do LOGOS nem teste de durabilidade/recall.
import { expect, test, afterAll } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";

const ref = process.env.LOGOS_UNIICHAT_REFERENCE;
if (!ref) throw new Error("Defina LOGOS_UNIICHAT_REFERENCE para o clone público fixado.");
const evidence = JSON.parse(fs.readFileSync(new URL("./uniichat-evidence.json", import.meta.url), "utf8"));
const revision = Bun.spawnSync(["git", "-C", ref, "rev-parse", "HEAD"]);
expect(revision.exitCode).toBe(0);
expect(revision.stdout.toString().trim()).toBe(evidence.uniichat.commit);
expect(Bun.spawnSync(["git", "-C", ref, "diff", "--exit-code", "HEAD", "--"]).exitCode).toBe(0);
for (const [file, hash] of Object.entries(evidence.uniichat.sha256)) {
  expect(createHash("sha256").update(fs.readFileSync(path.join(ref, file))).digest("hex")).toBe(hash);
}
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "logos-unii-reference-"));
process.env.UNII_DIR = dir;
const { Memory } = await import(pathToFileURL(path.join(ref, "Memory.ts")).href);
const { AskAI } = await import(pathToFileURL(path.join(ref, "AskAI.ts")).href);
const { Disk } = await import(pathToFileURL(path.join(ref, "Disk.ts")).href);
const share = { vendor: "claude-api", model: "reference-test", effort: "medium", speed: "slow", system: "fixed", searches: 0, server: "" };
const opened: any[] = [];
function memory(name: string): any {
  const mem = Memory.load(path.join(dir, name), () => share);
  mem.done = true; // só os jobs explicitamente testados; nunca iniciar chamadas de fundo
  opened.push(mem);
  return mem;
}
function nodes(mem: any, count: number, text = "x".repeat(100)): void {
  mem.count = count;
  for (let l = 0; 2 ** l <= count; l++) {
    for (let i = 0; (i + 1) * 2 ** l <= count; i++) mem.nodes.set(Memory.slot(l, i), text);
  }
}
function call(text: string, ok = true, stop?: string): any {
  return { done: Promise.resolve({ text, ok, stop, left: [] }), send() {}, stop() {}, move() {} };
}
afterAll(async () => {
  opened.forEach((mem) => Memory.stop(mem));
  await Disk.synced();
});

test("free nodes preservam a fonte; pai usa exatamente os dois filhos", () => {
  const mem = memory("free");
  Memory.push(mem, [{ kind: "user", text: "one" }, { kind: "unii", text: "two" }]);
  expect(Memory.source(mem, 0, 0)).toBe("user: one");
  mem.done = false;
  Memory.pump(mem);
  mem.done = true;
  expect(Memory.node(mem, { l: 0, i: 0 })).toBe("user: one");
  expect(Memory.node(mem, { l: 1, i: 0 })).toBe("user: one\nunii: two");
  expect(mem.calls.size).toBe(0);
});

test("due em H=8 escolhe 4+1/5+1 antes de 0+2/2+2", () => {
  const mem = memory("due");
  nodes(mem, 8);
  const parts = [{ l: 1, i: 0 }, { l: 1, i: 1 }, ...[4, 5, 6, 7].map((i) => ({ l: 0, i }))];
  const merged = Memory.merge(mem, { parts, size: 600, high: 1100, batch: false });
  expect(merged.parts.map(Memory.ref)).toEqual(["0+2", "2+2", "4+2", "6+1", "7+1"]);
  expect(merged.size).toBe(500);
});

test("fit preserva acima de low e abaixo de high; passou high, dobra até low", () => {
  const mem = memory("batch");
  nodes(mem, 16);
  mem.view = { parts: Array.from({ length: 6 }, (_, i) => ({ l: 0, i })), size: 600, high: 1000, batch: false };
  mem.count = 6;
  Memory.fit(mem);
  expect(mem.view.parts).toHaveLength(6);
  mem.count = 16;
  Memory.fit(mem);
  expect(mem.view.size).toBeLessThanOrEqual(500);
  expect(mem.view.batch).toBe(false);
  expect(mem.view.parts.length).toBeLessThan(10);
});

test("view usa blocos de quatro linhas e só marca o último bloco completo", () => {
  const mem = memory("blocks");
  nodes(mem, 9, "line");
  const parts = Array.from({ length: 9 }, (_, i) => ({ l: 0, i }));
  const seven = Memory.view(mem, 7, parts);
  const nine = Memory.view(mem, 9, parts);
  expect(seven[1].text).toBe(nine[1].text);
  expect(seven.map((b: any) => b.mark)).toEqual([false, true, false]);
  expect(nine.map((b: any) => b.mark)).toEqual([false, false, true, false]);
  expect(nine.at(-1).text).toBe("8+1|line\n</chat>\n");
  expect(Memory.view(mem, 3, parts).map((b: any) => b.mark)).toEqual([true, false]);
  const previousKeys = AskAI.keys({ ...share, blocks: seven });
  expect(AskAI.keys({ ...share, blocks: nine }).slice(0, previousKeys.length)).toEqual(previousKeys);
});

test("contexto de pai pode incluir seus filhos mais finos; não inclui eventos posteriores", async () => {
  const mem = memory("context");
  nodes(mem, 4, "detail");
  mem.nodes.delete(Memory.slot(1, 0)); // o pai em construção ainda não existe
  mem.seen.parts = Array.from({ length: 4 }, (_, i) => ({ l: 0, i }));
  expect(Memory.ctx(0, 3)).toBe(3);
  expect(Memory.ctx(1, 0)).toBe(2);
  const real = AskAI.ask;
  let blocks: any[] = [];
  AskAI.ask = (ask: any) => { blocks = ask.blocks; return call("summary"); };
  try { await Memory.build(mem, 1, 0, "left\nright", null); }
  finally { AskAI.ask = real; }
  const frame = blocks.slice(0, -1).map((b: any) => b.text).join("");
  expect(frame).toContain("0+1|detail");
  expect(frame).toContain("1+1|detail");
  expect(frame).not.toContain("2+1|");
  expect(blocks.at(-1).mark).toBe(false);
});

test("cinco respostas oversized guardam a menor; retries não acumulam transcript", async () => {
  const mem = memory("oversized");
  Memory.push(mem, [{ kind: "user", text: "source ".repeat(200) }]);
  const real = AskAI.ask;
  const asks: any[] = [];
  const lengths = [800, 700, 600, 513];
  AskAI.ask = (ask: any) => { asks.push(ask); return call("x".repeat(lengths[asks.length - 1])); };
  mem.done = false;
  try { await Memory.build(mem, 0, 0, Memory.source(mem, 0, 0), call("x".repeat(900))); }
  finally { AskAI.ask = real; mem.done = true; }
  expect(asks).toHaveLength(4);
  expect(asks.map((a) => a.blocks.length)).toEqual([4, 4, 4, 4]);
  expect(asks.every((a) => a.blocks.at(-1).text.startsWith("Too long:"))).toBe(true);
  expect(asks.every((a) => a.blocks.at(-1).mark === false)).toBe(true);
  expect(Memory.bytes(Memory.node(mem, { l: 0, i: 0 }))).toBe(513);
});

test("recusa recorta fonte; indisponibilidade sem resposta não publica node", async () => {
  const refused = memory("refused");
  Memory.push(refused, [{ kind: "user", text: "á".repeat(600) }]);
  const source = Memory.source(refused, 0, 0);
  await Memory.build(refused, 0, 0, source, call("", false, "fail"));
  expect(Memory.node(refused, { l: 0, i: 0 })).toBe(Memory.cut(Memory.flat(source)));
  expect(Memory.bytes(Memory.node(refused, { l: 0, i: 0 }))).toBeLessThanOrEqual(512);
  const down = memory("down");
  Memory.push(down, [{ kind: "user", text: "long".repeat(200) }]);
  await expect(Memory.build(down, 0, 0, Memory.source(down, 0, 0), call("down", false, "down"))).rejects.toThrow("down");
  expect(Memory.node(down, { l: 0, i: 0 })).toBeNull();
});

test("single flight libera seguidor no início da resposta e em falha do líder", async () => {
  const ask = (prefix: string): any => ({ ...share, blocks: [{ text: prefix, mark: true }, { text: "task", mark: false }] });
  const leader = await AskAI.wait(ask("unique prefix 1"));
  let ready = false;
  const waiter = AskAI.wait(ask("unique prefix 1")).then((flight: any) => { ready = true; return flight; });
  await new Promise((ok) => setImmediate(ok));
  expect(ready).toBe(false);
  leader.begun();
  const follower = await waiter;
  expect(ready).toBe(true); // antes de leader.end(), não espera geração completa
  leader.end(); follower.end();
  const failed = await AskAI.wait(ask("unique prefix 2"));
  const released = AskAI.wait(ask("unique prefix 2"));
  failed.end();
  const retry = await released;
  retry.begun(); retry.end();
  expect(AskAI.keys({ ...share, blocks: [{ text: "only last", mark: true }] })).toEqual([]);
});
