# Planilha de revisão humana — Etapa 9.1

Gerada por `cargo run -p logos-seal --example review_sheet`; não editar à mão
as paráfrases. Gramática `paraphrase-pt/v1`, renderizador `logos-seal/render_pt/1`,
`h(V)` = `921c6c0d78894d3a4ab9d61f57f614a8ee593b43cb8e8a361575602107fcbcca`. Leia a paráfrase **como está escrita** e compare com a
intenção: a leitura só é correta se as duas dizem a mesma coisa, inclusive
escopo dos quantificadores, negações e condições. Registre o resultado no
[README](README.md#registro-da-revisão).

## P01

**Intenção:** Qualquer pessoa pode ler um recurso público.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x2⟧ tem acesso público⟧), então (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `e844b606549fc165f89cd6c90da24bd2dce77c30fde89f40738478363dda9733`

Leitura correta: ☐ sim ☐ não — observações:

## P02

**Intenção:** O dono de um recurso pode escrever nele.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (o valor ⟦o dono de ⟦x2⟧⟧ é igual a ⟦x1⟧), então (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `63da08877660b5c5b1cc861d51e9cbe25978e3cd74432760a2d00a9e9d832a97`

Leitura correta: ☐ sim ☐ não — observações:

## P03

**Intenção:** Administradores podem apagar qualquer recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧), então (⟦⟦x1⟧ pode ⟦apagar⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `675655fc4dfcd3d58216b9494943ce015ea5d72ef7edfe1059629477c9d471e9`

Leitura correta: ☐ sim ☐ não — observações:

## P04

**Intenção:** Colaboradores externos não podem ler recursos que contenham dados pessoais sensíveis.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x2⟧ contém dados pessoais sensíveis⟧); (⟦⟦x1⟧ tem vínculo externo⟧)), então (não é o caso que (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `dc8a55b6643e3410183d6794bdd7b9d1141e0c1549f5918f758766d379dddb28`

Leitura correta: ☐ sim ☐ não — observações:

## P05

**Intenção:** Nenhum recurso sob retenção legal pode ser apagado, por ninguém.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x2⟧ está sob retenção legal⟧), então (não é o caso que (⟦⟦x1⟧ pode ⟦apagar⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `1f7b656d1194dd5a0cfc5c82c8ad7c48078dc1422e1573905b300003fd7957c3`

Leitura correta: ☐ sim ☐ não — observações:

## P06

**Intenção:** Só quem tem conta ativa pode escrever em algum recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧), então (⟦⟦x1⟧ tem conta ativa⟧)))
```

`h(P)` = `bdfc6c996c6f5c2c999cd077bc38da2719909f47da4fadf0242e65190d2102e4`

Leitura correta: ☐ sim ☐ não — observações:

## P07

**Intenção:** Todo administrador usa autenticação de dois fatores.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧), então (⟦⟦x1⟧ usa autenticação de dois fatores⟧))
```

`h(P)` = `bfbbf0db7b73a96637fc72718a8e859e71de2d44be1e95bb0368923aecadb2fd`

Leitura correta: ☐ sim ☐ não — observações:

## P08

**Intenção:** Só auditores podem exportar recursos com dados sensíveis.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x2⟧ contém dados pessoais sensíveis⟧); (⟦⟦x1⟧ pode ⟦exportar⟧ o recurso ⟦x2⟧⟧)), então (⟦⟦x1⟧ tem ⟦o papel de auditor⟧⟧)))
```

`h(P)` = `4e9dc1d45bd24c40d8e5e69dc543350bc07b7d7c5fda62b15af12b5872fab165`

Leitura correta: ☐ sim ☐ não — observações:

## P09

**Intenção:** Recurso com dados pessoais sensíveis nunca é público.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ contém dados pessoais sensíveis⟧), então (não é o caso que (⟦⟦x1⟧ tem acesso público⟧)))
```

`h(P)` = `c1d566095f6e0b4ce851770f38eeda4c6d1f260b9715e709142ab57c3c91c7ab`

Leitura correta: ☐ sim ☐ não — observações:

## P10

**Intenção:** Ninguém escreve em recurso arquivado.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x2⟧ está em arquivo⟧), então (não é o caso que (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `b58deb9f0f2489a136139a9439e5b8b879dbf1b71ca4db6d755b0f428a7d6d8d`

Leitura correta: ☐ sim ☐ não — observações:

## P11

**Intenção:** Durante manutenção, só administradores podem escrever.

```text
se (⟦o sistema está em manutenção⟧), então (para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (não é o caso que (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧)), então (não é o caso que (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧)))))
```

`h(P)` = `fd3f8449fae22ee153f47d261ed4da2d3e72c90a59608333001afa33748ea8c4`

Leitura correta: ☐ sim ☐ não — observações:

## P12

**Intenção:** Só o dono ou um administrador pode compartilhar um recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦compartilhar⟧ o recurso ⟦x2⟧⟧), então (vale pelo menos uma de 2 condições: (o valor ⟦o dono de ⟦x2⟧⟧ é igual a ⟦x1⟧); (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧))))
```

`h(P)` = `211d4157eaf4ac3572151f365f29e1d5ebcb16d4cc6cd2df3d67a5552fe2f1b8`

Leitura correta: ☐ sim ☐ não — observações:

## P13

**Intenção:** Colaboradores externos nunca são administradores.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem vínculo externo⟧), então (não é o caso que (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧)))
```

`h(P)` = `5d48f1b44852a0c50d5edad445283d8157cc3d4f007fe3398aaaeeab910ddbeb`

Leitura correta: ☐ sim ☐ não — observações:

## P14

**Intenção:** Todo membro de uma equipe tem conta ativa.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Equipe⟧, vale que (se (⟦⟦x1⟧ participa de ⟦x2⟧⟧), então (⟦⟦x1⟧ tem conta ativa⟧)))
```

`h(P)` = `a4ebc075b4db57f471529ee605bf0f2297b15610f58156e0f6674e52dbe41f29`

Leitura correta: ☐ sim ☐ não — observações:

## P15

**Intenção:** O responsável por uma equipe é membro dela.

```text
para todo x1 do tipo ⟦Equipe⟧, vale que (⟦⟦o responsável por ⟦x1⟧⟧ participa de ⟦x1⟧⟧)
```

`h(P)` = `095302c8e72e52292ab21e634ba4baef8d3100e7a1ba87af7d4bf702705184dc`

Leitura correta: ☐ sim ☐ não — observações:

## P16

**Intenção:** Membros da equipe do dono de um recurso podem lê-lo.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ participa de ⟦a equipe de ⟦o dono de ⟦x2⟧⟧⟧⟧), então (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `3d72bba1735c9d5426d2b8012ba2718ed5b7908d49cd0f0c26a072ab0af76e7a`

Leitura correta: ☐ sim ☐ não — observações:

## P17

**Intenção:** Todo recurso sensível tem consentimento registrado de alguém.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ contém dados pessoais sensíveis⟧), então (existe x2 do tipo ⟦Pessoa⟧ tal que (⟦⟦x2⟧ consentiu com o uso de ⟦x1⟧⟧)))
```

`h(P)` = `05b1ba0ef2782811d345cebd84fd57fab9a574787877808e69a3fb3c8393f2a6`

Leitura correta: ☐ sim ☐ não — observações:

## P18

**Intenção:** Ler um recurso sensível exige que o dono dele tenha consentido com o uso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x2⟧ contém dados pessoais sensíveis⟧); (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)), então (⟦⟦o dono de ⟦x2⟧⟧ consentiu com o uso de ⟦x2⟧⟧)))
```

`h(P)` = `8c0dd05fc421da1505668ddc19936911e5012957446fb32a6c45164e80c496f8`

Leitura correta: ☐ sim ☐ não — observações:

## P19

**Intenção:** Todo recurso público teve a revisão aprovada por um professor.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ tem acesso público⟧), então (existe x2 do tipo ⟦Pessoa⟧ tal que (vale cada uma de 2 condições: (⟦⟦x2⟧ aprovou a revisão de ⟦x1⟧⟧); (⟦⟦x2⟧ tem ⟦o papel de professor⟧⟧))))
```

`h(P)` = `13ac2801353bd9f2b703af9117204ff268acfab56ba53e67ef8e11188c565e8b`

Leitura correta: ☐ sim ☐ não — observações:

## P20

**Intenção:** Alunos não podem apagar recursos.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de aluno⟧⟧), então (não é o caso que (⟦⟦x1⟧ pode ⟦apagar⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `64ae4e6eec73c6211f48d8c72efb016709fa10d5e46ce95a3a941bb5ad0da7c2`

Leitura correta: ☐ sim ☐ não — observações:

## P21

**Intenção:** Só um administrador ou o próprio dono pode apagar um recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦apagar⟧ o recurso ⟦x2⟧⟧), então (vale pelo menos uma de 2 condições: (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧); (o valor ⟦o dono de ⟦x2⟧⟧ é igual a ⟦x1⟧))))
```

`h(P)` = `674a5df3bdcbdf52e3a4f9e378e0ed718e2f8f31df17dba6b8de4e216c5f8eed`

Leitura correta: ☐ sim ☐ não — observações:

## P22

**Intenção:** Ninguém aprova a revisão de um recurso do qual é dono.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ aprovou a revisão de ⟦x2⟧⟧), então (não é o caso que (o valor ⟦o dono de ⟦x2⟧⟧ é igual a ⟦x1⟧))))
```

`h(P)` = `99120baece5641c7c3044a9fb1a32ae7a4b30559cd48c68b5f84ee28c96e1afc`

Leitura correta: ☐ sim ☐ não — observações:

## P23

**Intenção:** Durante um incidente de segurança, contas de colaboradores externos ficam inativas.

```text
se (⟦há um incidente de segurança declarado⟧), então (para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem vínculo externo⟧), então (não é o caso que (⟦⟦x1⟧ tem conta ativa⟧))))
```

`h(P)` = `95d8f94d2dfd348051044c54573000ef30fe46f4cba7fc2fd9174a8fc8d73183`

Leitura correta: ☐ sim ☐ não — observações:

## P24

**Intenção:** Quem pode ler uma pasta pode ler o que está dentro dela.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦a pasta que contém ⟦x2⟧⟧⟧), então (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `1d3fd133211eca2d1a74696830910437ad26448ad87da679187f19becc72716c`

Leitura correta: ☐ sim ☐ não — observações:

## P25

**Intenção:** Um recurso é público exatamente quando a pasta que o contém é pública.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que ((⟦⟦x1⟧ tem acesso público⟧) se e somente se (⟦⟦a pasta que contém ⟦x1⟧⟧ tem acesso público⟧))
```

`h(P)` = `5bfa9f6b876cecad193c953270e08ca8e6e273db799b45b6bcb2c04b5274424e`

Leitura correta: ☐ sim ☐ não — observações:

## P26

**Intenção:** Quem pode escrever em um recurso também pode lê-lo.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧), então (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `fce1fbe86a0da04caf2f7daa5348281eb04cf39a810bbc566c7486e343ef0263`

Leitura correta: ☐ sim ☐ não — observações:

## P27

**Intenção:** Auditores podem ler todos os recursos.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de auditor⟧⟧), então (para todo x2 do tipo ⟦Recurso⟧, vale que (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)))
```

`h(P)` = `443a75c70726d8d057789cba797cf1d8720360869777f8140a67b60c81dbea21`

Leitura correta: ☐ sim ☐ não — observações:

## P28

**Intenção:** Auditores não podem escrever em nenhum recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de auditor⟧⟧), então (para todo x2 do tipo ⟦Recurso⟧, vale que (não é o caso que (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `dfa6e3392b970ae2c01e7f03911e4003bced5cd9faa749c01435ec7a7a1d046d`

Leitura correta: ☐ sim ☐ não — observações:

## P29

**Intenção:** Ninguém é aluno e professor ao mesmo tempo.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (não é o caso que (vale cada uma de 2 condições: (⟦⟦x1⟧ tem ⟦o papel de aluno⟧⟧); (⟦⟦x1⟧ tem ⟦o papel de professor⟧⟧)))
```

`h(P)` = `0b8ccda07f11158be714ff2639366f067f2578c7f3d7ce02abd0f0e331347d78`

Leitura correta: ☐ sim ☐ não — observações:

## P30

**Intenção:** Todo administrador faz parte da equipe de TI.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧), então (⟦⟦x1⟧ participa de ⟦a equipe de TI⟧⟧))
```

`h(P)` = `3b26550ac5dfe1827bbbe8c0da60f4efb1f33e3e265097130476d0f0f06ebdbe`

Leitura correta: ☐ sim ☐ não — observações:

## P31

**Intenção:** Um recurso sob retenção legal não fica arquivado.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ está sob retenção legal⟧), então (não é o caso que (⟦⟦x1⟧ está em arquivo⟧)))
```

`h(P)` = `3fdc04108cd8160cf78930312ed69f913372e0178c50f07086381233d57f6d81`

Leitura correta: ☐ sim ☐ não — observações:

## P32

**Intenção:** Recurso sob retenção legal só pode ser lido pela equipe jurídica ou por auditores.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x2⟧ está sob retenção legal⟧); (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧)), então (vale pelo menos uma de 2 condições: (⟦⟦x1⟧ participa de ⟦a equipe jurídica⟧⟧); (⟦⟦x1⟧ tem ⟦o papel de auditor⟧⟧))))
```

`h(P)` = `34b4ad3b6820861d67eceda7813d0bdf6d8e4da5cc1ee0681cae126ec94e6f98`

Leitura correta: ☐ sim ☐ não — observações:

## P33

**Intenção:** Existe pelo menos um administrador.

```text
existe x1 do tipo ⟦Pessoa⟧ tal que (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧)
```

`h(P)` = `98ea13569035ced145c84e0d5de3db8a51bb61705842f9816f5750ee2abc37fc`

Leitura correta: ☐ sim ☐ não — observações:

## P34

**Intenção:** Toda equipe tem ao menos um membro com conta ativa.

```text
para todo x1 do tipo ⟦Equipe⟧, vale que (existe x2 do tipo ⟦Pessoa⟧ tal que (vale cada uma de 2 condições: (⟦⟦x2⟧ participa de ⟦x1⟧⟧); (⟦⟦x2⟧ tem conta ativa⟧)))
```

`h(P)` = `f1ef4b55be5317db60a9c9f20a74e996cbcde5afab9ae2c2e2d00a13db66612a`

Leitura correta: ☐ sim ☐ não — observações:

## P35

**Intenção:** Cada recurso tem um único dono.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Pessoa⟧, vale que (para todo x3 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (o valor ⟦o dono de ⟦x3⟧⟧ é igual a ⟦x1⟧); (o valor ⟦o dono de ⟦x3⟧⟧ é igual a ⟦x2⟧)), então (o valor ⟦x1⟧ é igual a ⟦x2⟧))))
```

`h(P)` = `115fd4246fe3d027613997823904a2618cbcd0a649d41be351440a42f676cfca`

Leitura correta: ☐ sim ☐ não — observações:

## P36

**Intenção:** Se um colaborador externo pode escrever em um recurso, alguém aprovou a revisão desse recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x1⟧ tem vínculo externo⟧); (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧)), então (existe x3 do tipo ⟦Pessoa⟧ tal que (⟦⟦x3⟧ aprovou a revisão de ⟦x2⟧⟧))))
```

`h(P)` = `d8375d0eced03735e44830d448b2bfc7b605f0119285e99a69de979179e0af21`

Leitura correta: ☐ sim ☐ não — observações:

## P37

**Intenção:** Colaboradores externos não exportam nenhum recurso.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦exportar⟧ o recurso ⟦x2⟧⟧), então (não é o caso que (⟦⟦x1⟧ tem vínculo externo⟧))))
```

`h(P)` = `2dc5918ce6b05c3f46e5b5b03edbdd48dfb0678af7201535cdbd3e17592d495f`

Leitura correta: ☐ sim ☐ não — observações:

## P38

**Intenção:** Todo colaborador externo com conta ativa usa autenticação de dois fatores.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x1⟧ tem vínculo externo⟧); (⟦⟦x1⟧ tem conta ativa⟧)), então (⟦⟦x1⟧ usa autenticação de dois fatores⟧))
```

`h(P)` = `845b3875b90a4379b277405c3dcd72dcc2f1794f2b55e217c825c3573c35496e`

Leitura correta: ☐ sim ☐ não — observações:

## P39

**Intenção:** Qualquer leitura ou escrita exige conta ativa.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale pelo menos uma de 2 condições: (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧); (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧)), então (⟦⟦x1⟧ tem conta ativa⟧)))
```

`h(P)` = `fe0a4dc679fa4f881265137a302720e06741ea5036f18a2d358c91481ebddf50`

Leitura correta: ☐ sim ☐ não — observações:

## P40

**Intenção:** Manutenção e incidente de segurança nunca são declarados ao mesmo tempo.

```text
não é o caso que (vale cada uma de 2 condições: (⟦o sistema está em manutenção⟧); (⟦há um incidente de segurança declarado⟧))
```

`h(P)` = `49a91191f9e457da63c1c61488537033ea88afd1526c86cf60d0947faa8c16ff`

Leitura correta: ☐ sim ☐ não — observações:

## P41

**Intenção:** A pasta que contém um recurso sensível também é sensível.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ contém dados pessoais sensíveis⟧), então (⟦⟦a pasta que contém ⟦x1⟧⟧ contém dados pessoais sensíveis⟧))
```

`h(P)` = `b5aaec0dd600acf978dea833d43215cc17ce16f7960559e78fe24c5a83a004e6`

Leitura correta: ☐ sim ☐ não — observações:

## P42

**Intenção:** Nenhum recurso sensível pode ser compartilhado.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x1⟧ pode ⟦compartilhar⟧ o recurso ⟦x2⟧⟧); (⟦⟦x2⟧ contém dados pessoais sensíveis⟧)), então (falso)))
```

`h(P)` = `a3562fe9c002d0be06d99227ce4a3b94f97317f190b172d4313655336771e1a7`

Leitura correta: ☐ sim ☐ não — observações:

## P43

**Intenção:** Professores não são colaboradores externos.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (se (⟦⟦x1⟧ tem ⟦o papel de professor⟧⟧), então (não é o caso que (⟦⟦x1⟧ tem vínculo externo⟧)))
```

`h(P)` = `42121960b2ca87f304051bc8c2f3fa15ef95f172e86b75ae4db0410551a3aa44`

Leitura correta: ☐ sim ☐ não — observações:

## P44

**Intenção:** Alunos não leem recursos com dados sensíveis.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x1⟧ tem ⟦o papel de aluno⟧⟧); (⟦⟦x2⟧ contém dados pessoais sensíveis⟧)), então (não é o caso que (⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧))))
```

`h(P)` = `68759ce46ceb34733722310734820f0aa54c0aded178ffa15a358043dfdb2976`

Leitura correta: ☐ sim ☐ não — observações:

## P45

**Intenção:** O responsável por uma equipe é professor ou administrador.

```text
para todo x1 do tipo ⟦Equipe⟧, vale que (vale pelo menos uma de 2 condições: (⟦⟦o responsável por ⟦x1⟧⟧ tem ⟦o papel de professor⟧⟧); (⟦⟦o responsável por ⟦x1⟧⟧ tem ⟦o papel de administrador⟧⟧))
```

`h(P)` = `d234f36ed6eb386541b9a9a2696c68794a9f6865a9c07f1614cc291c175b4607`

Leitura correta: ☐ sim ☐ não — observações:

## P46

**Intenção:** Recurso arquivado que continua público precisa de revisão aprovada por um auditor.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (se (vale cada uma de 2 condições: (⟦⟦x1⟧ está em arquivo⟧); (⟦⟦x1⟧ tem acesso público⟧)), então (existe x2 do tipo ⟦Pessoa⟧ tal que (vale cada uma de 2 condições: (⟦⟦x2⟧ tem ⟦o papel de auditor⟧⟧); (⟦⟦x2⟧ aprovou a revisão de ⟦x1⟧⟧))))
```

`h(P)` = `32c27def5bbf0e9f5b299f09060b24d7719f0a45610a81d0ca40e9b70826559a`

Leitura correta: ☐ sim ☐ não — observações:

## P47

**Intenção:** Só se apaga recurso que não está sob retenção legal.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦apagar⟧ o recurso ⟦x2⟧⟧), então (não é o caso que (⟦⟦x2⟧ está sob retenção legal⟧))))
```

`h(P)` = `9e36ffdb5686acbbfc15e13fee6c127ea3676344628078a2772e921a0a5c30fb`

Leitura correta: ☐ sim ☐ não — observações:

## P48

**Intenção:** Durante incidente de segurança, só administradores escrevem.

```text
se (⟦há um incidente de segurança declarado⟧), então (para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que (se (⟦⟦x1⟧ pode ⟦escrever⟧ o recurso ⟦x2⟧⟧), então (⟦⟦x1⟧ tem ⟦o papel de administrador⟧⟧))))
```

`h(P)` = `c5b4c6d843be6318d4cbd9ba0ce8dfa78961973301554af7004ae54afe4cbb9d`

Leitura correta: ☐ sim ☐ não — observações:

## P49

**Intenção:** Nenhum recurso pertence a um colaborador externo.

```text
para todo x1 do tipo ⟦Recurso⟧, vale que (não é o caso que (⟦⟦o dono de ⟦x1⟧⟧ tem vínculo externo⟧))
```

`h(P)` = `710df430f528690e1137ac7d3893333e8c4e3401daac6034711b2c03b7fea798`

Leitura correta: ☐ sim ☐ não — observações:

## P50

**Intenção:** Ler um recurso é permitido exatamente ao dono, a auditores e, se o recurso for público, a todos.

```text
para todo x1 do tipo ⟦Pessoa⟧, vale que (para todo x2 do tipo ⟦Recurso⟧, vale que ((⟦⟦x1⟧ pode ⟦ler⟧ o recurso ⟦x2⟧⟧) se e somente se (vale pelo menos uma de 2 condições: (vale pelo menos uma de 2 condições: (⟦⟦x2⟧ tem acesso público⟧); (o valor ⟦o dono de ⟦x2⟧⟧ é igual a ⟦x1⟧)); (⟦⟦x1⟧ tem ⟦o papel de auditor⟧⟧))))
```

`h(P)` = `a0de020781309b17d9fb28a77b98e9972056f1701071178a7555897c0414fda1`

Leitura correta: ☐ sim ☐ não — observações:

