"""Domínio piloto da 5R.3: qualidade da água de um igarapé (fixture sintética).

Tudo aqui é material de teste: nenhuma fonte é literatura real, nenhuma
medição foi feita. A proveniência diz isso explicitamente em cada item.
"""

from __future__ import annotations

from dataclasses import dataclass

RECORDED_AT = 1_792_627_200  # 2026-10-22T00:00:00Z
SCOPE = "session:piloto-igarape"
LAWS_SCOPE = "session:piloto-igarape-leis"
SOURCE = "fixture sintética do piloto 5R.3 (não é literatura nem medição real)"
ACTOR = "pesquisador-fixture"

VOCABULARY = """vocab v1 {
  pred ChuvaForte() gloss "choveu forte nas 48 h anteriores à coleta"
  pred Esgoto() gloss "há lançamento de esgoto a montante do ponto"
  pred Desmatamento() gloss "a mata ciliar do trecho está desmatada"
  pred Nutrientes() gloss "nitrogênio/fósforo acima do limite de referência"
  pred Calor() gloss "temperatura da água acima de 30 °C"
  pred Algas() gloss "floração de algas visível"
  pred Turbidez() gloss "turbidez acima do limite de referência"
  pred OxigenioBaixo() gloss "oxigênio dissolvido abaixo de 4 mg/L"
  pred Mortandade() gloss "peixes mortos observados"
  pred Coliformes() gloss "coliformes termotolerantes acima do limite"
  pred Assoreamento() gloss "assoreamento visível no leito"
  pred Odor() gloss "odor forte de esgoto"
}"""

OBSERVABLE = [
    "ChuvaForte",
    "Esgoto",
    "Desmatamento",
    "Calor",
    "Algas",
    "Turbidez",
    "OxigenioBaixo",
    "Mortandade",
    "Coliformes",
    "Assoreamento",
    "Odor",
]  # Nutrientes exige análise laboratorial: não é observável em campo aqui.


@dataclass(frozen=True)
class Item:
    key: str
    title: str
    statement: str
    formula: str | None
    method: str


METHOD_H = "formulação manual a partir de conhecimento geral (não revisada por especialista)"

HYPOTHESES = [
    Item(
        "H01",
        "Esgoto eleva nutrientes",
        "Esgoto a montante eleva nutrientes.",
        "Esgoto() -> Nutrientes()",
        METHOD_H,
    ),
    Item(
        "H02",
        "Nutrientes e calor causam floração",
        "Com nutrientes altos e água quente, há floração de algas.",
        "Nutrientes() and Calor() -> Algas()",
        METHOD_H,
    ),
    Item(
        "H03",
        "Floração derruba oxigênio",
        "Floração de algas derruba o oxigênio.",
        "Algas() -> OxigenioBaixo()",
        METHOD_H,
    ),
    Item(
        "H04",
        "Oxigênio baixo mata peixes",
        "Oxigênio baixo causa mortandade de peixes.",
        "OxigenioBaixo() -> Mortandade()",
        METHOD_H,
    ),
    Item(
        "H05",
        "Chuva em trecho desmatado turva a água",
        "Chuva forte em trecho desmatado aumenta a turbidez.",
        "ChuvaForte() and Desmatamento() -> Turbidez()",
        METHOD_H,
    ),
    Item(
        "H06",
        "Turbidez derruba oxigênio",
        "Turbidez alta derruba o oxigênio.",
        "Turbidez() -> OxigenioBaixo()",
        METHOD_H,
    ),
    Item(
        "H07",
        "Esgoto traz coliformes",
        "Esgoto a montante eleva coliformes.",
        "Esgoto() -> Coliformes()",
        METHOD_H,
    ),
    Item(
        "H08",
        "Desmatamento assoreia",
        "Desmatamento ciliar causa assoreamento.",
        "Desmatamento() -> Assoreamento()",
        METHOD_H,
    ),
    Item(
        "H09",
        "Esgoto causa odor",
        "Esgoto a montante causa odor forte.",
        "Esgoto() -> Odor()",
        METHOD_H,
    ),
    Item(
        "H10",
        "Leito assoreado turva com chuva",
        "Com assoreamento e chuva forte, a turbidez sobe.",
        "Assoreamento() and ChuvaForte() -> Turbidez()",
        METHOD_H,
    ),
    Item(
        "H11",
        "Floração turva a água",
        "Floração de algas aumenta a turbidez.",
        "Algas() -> Turbidez()",
        METHOD_H,
    ),
    Item(
        "H12",
        "Oxigênio baixo vem só da floração",
        "Quando o oxigênio está baixo, há floração e não há turbidez.",
        "OxigenioBaixo() -> Algas() and not Turbidez()",
        METHOD_H,
    ),
    Item(
        "H13",
        "Oxigênio baixo vem só de sedimentos",
        "Quando o oxigênio está baixo, há turbidez e não há floração.",
        "OxigenioBaixo() -> Turbidez() and not Algas()",
        METHOD_H,
    ),
    Item(
        "H14",
        "Mortandade indica oxigênio baixo",
        "Mortandade de peixes indica oxigênio baixo.",
        "Mortandade() -> OxigenioBaixo()",
        METHOD_H,
    ),
    Item(
        "H15",
        "Coliformes indicam esgoto",
        "Coliformes altos indicam esgoto a montante.",
        "Coliformes() -> Esgoto()",
        METHOD_H,
    ),
    Item(
        "H16",
        "Nutrientes bastam para floração",
        "Nutrientes altos bastam para floração.",
        "Nutrientes() -> Algas()",
        METHOD_H,
    ),
    Item(
        "H17",
        "Calor derruba oxigênio",
        "Água quente derruba o oxigênio.",
        "Calor() -> OxigenioBaixo()",
        METHOD_H,
    ),
    Item(
        "H18",
        "Chuva forte turva a água",
        "Chuva forte sempre aumenta a turbidez.",
        "ChuvaForte() -> Turbidez()",
        METHOD_H,
    ),
    Item(
        "H19",
        "Odor indica esgoto",
        "Odor forte indica esgoto a montante.",
        "Odor() -> Esgoto()",
        METHOD_H,
    ),
    Item(
        "H20",
        "Desmatamento aquece a água",
        "Desmatamento ciliar aquece a água.",
        "Desmatamento() -> Calor()",
        METHOD_H,
    ),
]

# Evidências: com fórmula viram assumption explícita (5.7); sem fórmula são notas.
EVIDENCE = [
    Item(
        "E1",
        "Tubulação de esgoto ativa a montante",
        "Inspeção registrou tubulação de esgoto ativa a montante do ponto P3.",
        "Esgoto()",
        "inspeção visual de campo (fictícia)",
    ),
    Item(
        "E2",
        "Água acima de 30 °C",
        "Termômetro de campo registrou 31 °C no ponto P3.",
        "Calor()",
        "termômetro de campo (leitura fictícia)",
    ),
    Item(
        "E3",
        "Chuva forte antes da coleta",
        "Pluviômetro registrou chuva forte nas 48 h anteriores.",
        "ChuvaForte()",
        "pluviômetro local (leitura fictícia)",
    ),
    Item(
        "E4",
        "Mata ciliar desmatada",
        "Imagem de satélite indica mata ciliar removida no trecho.",
        "Desmatamento()",
        "interpretação de imagem (fonte fictícia)",
    ),
    Item(
        "E5",
        "Oxigênio dissolvido baixo",
        "Sonda multiparâmetro registrou 3,1 mg/L de oxigênio dissolvido.",
        "OxigenioBaixo()",
        "sonda multiparâmetro (leitura fictícia)",
    ),
    Item(
        "E6",
        "Relato de peixes mortos",
        "Morador relatou peixes mortos na semana anterior; não verificado.",
        None,
        "relato oral registrado (fictício)",
    ),
]

TABLE = b"""linha,Esgoto,Coliformes,Odor,Desmatamento,Assoreamento,ChuvaForte,Turbidez
P1,1,1,1,0,0,0,0
P2,1,1,1,1,1,1,1
P3,0,0,0,1,1,0,0
P4,1,1,0,0,0,0,0
P5,0,0,0,0,0,0,0
P6,0,0,0,0,0,1,0
P7,0,0,0,1,1,1,0
P8,1,1,1,1,1,0,1
"""

LAWS = [
    ("L1", "Esgoto() -> Coliformes()"),
    ("L2", "Esgoto() -> Odor()"),
    ("L3", "Desmatamento() -> Assoreamento()"),
    ("L4", "Esgoto() and Desmatamento() -> Coliformes()"),
    ("L5", "ChuvaForte() -> Turbidez()"),
    ("L6", "Coliformes() -> Esgoto()"),
    ("L7", "Desmatamento() and Esgoto() -> Assoreamento() and Coliformes()"),
]
