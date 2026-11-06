<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Fiduciais numa imagem de câmara: marcas, escala e correção de uma placa

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-vision`) com 128 testes que correm num computador: encontra marcas fiduciais redondas numa imagem, converte píxeis em milímetros e calcula a rotação e o deslocamento que põem uma placa onde diz o seu desenho. Tem uma janela (compila-se com `--features gui`, o programa `eh3d-vision-gui`) que abre uma imagem, mostra as marcas com as suas coordenadas, define a escala, ajusta a distorção da objetiva com uma grelha, alinha com o desenho e exporta as opções do ISOCAM, mas a janela ainda não foi vista num ecrã; não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.3` |
| Maturidade | Estrutura inicial |
| Testes | 128 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | encontra marcas fiduciais redondas numa imagem, converte píxeis em milímetros e calcula a rotação e o deslocamento que põem uma placa onde diz o seu desenho; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Linha de comandos | 🟡 | `eh3d-vision` executa as mesmas funções da biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 128 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🟠 | Escrita (egui, `--features gui`, o programa `eh3d-vision-gui`): abrir ou largar uma imagem, ver as marcas com as suas coordenadas, definir a escala com uma marca de tamanho conhecido, ajustar a distorção da objetiva com uma grelha, alinhar com um desenho CSV e exportar as opções do ISOCAM; a sua lógica está testada, mas ainda não foi compilada nem vista num ecrã. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **As marcas** (`src/fiducial.rs`): a imagem é dividida em escuro e claro com o limiar de Otsu, medem-se as manchas ligadas e as que são discos do tamanho esperado (a área, quanto enchem do círculo que as contém e quão alongadas são) são as marcas, as melhores primeiro; uma mancha cortada pela borda da imagem fica de fora porque não se pode confiar no seu centro, e as marcas podem ser escuras sobre claro ou claras sobre escuro.
* **A escala e a lente** (`src/scale.rs`): os milímetros de um píxel saem de uma marca de tamanho conhecido, os eixos da máquina sobem enquanto a imagem desce, e a distorção radial da lente recupera-se por mínimos quadrados a partir de uma grelha de marcas e corrige-se.
* **A placa** (`src/align.rs`): com dois ou mais fiduciais do desenho e os mesmos como a câmara os encontrou, calculam-se por mínimos quadrados a rotação, o deslocamento e (se permitido) o tamanho, com o resíduo em milímetros; uma transformação com resíduo demasiado grande (uma marca errada) ou com o tamanho desviado (uma placa esticada) não é aceite.

## 🧩 Papel no EH3D

EH3D-VISION-STUDIO é um programa da EH3D Fabrication Suite. Funciona como biblioteca e linha de comandos e, compilado com `--features gui`, como janela com o aspeto que os outros programas da suíte partilham. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-VISION-STUDIO: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `eh3d-vision` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Linha de comandos estrita:** uma opção mal escrita (`--powr`) ou uma palavra a mais é um erro que a nomeia, não algo que se ignora; caso contrário o programa faria a sua saída com os valores por omissão.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.
* **As marcas para o ISOCAM:** `align --isocam-args` escreve cada par de marcas como a opção `--fiducial X_PROJETO,Y_PROJETO,X_MEDIDO,Y_MEDIDO` do `eh3d-isocam`.
* **Uma janela:** compila-se com `--features gui` como o programa `eh3d-vision-gui` (no Windows abre sem consola): abre uma imagem PNG ou PGM com zoom e deslocamento, encontra as marcas com valores editáveis, desenha-as com as suas coordenadas, define a escala com uma marca de tamanho conhecido, ajusta a distorção da objetiva com uma grelha de marcas, alinha com um desenho de um ficheiro CSV (as marcas são emparelhadas com ele pela posição, por isso a ordem não importa) e exporta as opções `--fiducial` do ISOCAM só quando o alinhamento é aceite. Fala os oito idiomas e não precisa de rede.
* **Um ficheiro de configuração:** `vision.cfg` guarda cada valor afinado (o aspeto de uma marca, a grelha do alvo, os limites de um alinhamento aceite, o maior ficheiro lido); um erro nele é recusado com a sua linha e `examples/vision.cfg` enumera-os todos.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_vision_studio` (`use eh3d_vision_studio::...`); a linha de comandos é `eh3d-vision`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuração

Os valores que se afinam vivem num ficheiro simples de `chave = valor`, não no código: `vision.cfg` na pasta onde o programa arranca ou ao lado dele (a variável `EH3D_VISION_CONFIG` indica outro ficheiro). Define o idioma da janela, se as marcas são escuras ou claras, o limiar, o intervalo do diâmetro, o preenchimento e o alongamento de uma marca, o tamanho da marca que define a escala, as colunas e as linhas da grelha, se o alinhamento pode mudar o tamanho, os limites que um alinhamento tem de cumprir e o maior ficheiro lido. `examples/vision.cfg` enumera cada chave com o seu significado e o valor com que o programa arranca; se o ficheiro faltar usam-se esses valores e um ficheiro com um erro é recusado com a sua linha. Na linha de comandos as mesmas coisas dão-se como opções (ver Interfaces).

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.88 ou mais recente). O repositório EH3D-COMMON tem de estar clonado ao lado deste: o leitor de imagens é tomado dele por caminho.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
```

## 🧪 Testes

O conjunto tem 128 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

### Transferências

Cada versão traz o programa com a sua janela para Windows (64 e 32 bits), Linux (64 bits) e macOS (Apple silicon e Intel), como `<nome>-<versão>-<sistema>.zip` (Windows) ou `.tar.gz` com o programa, a licença, o leia-me, o ícone e os exemplos. Os ficheiros são construídos pelo GitHub Actions, constam de `SHA256SUMS` e estão assinados com a chave da EH3D. Para verificar uma transferência, compare o seu hash com o ficheiro `.sha256` que a acompanha (`sha256sum -c`) e verifique a assinatura com o ficheiro `.sig` que a acompanha (`ssh-keygen -Y verify`).

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── gui_main.rs                              a janela como programa à parte (funcionalidade gui)
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── align.rs                                 a rotação, o deslocamento e o tamanho de uma placa e o seu emparelhamento com o desenho
│   ├── config.rs                                o ficheiro de configuração: cada valor afinado
│   ├── fiducial.rs                              as marcas redondas de uma imagem: limiar, manchas e verificações
│   ├── grid.rs                                  a distorção da objetiva a partir de uma grelha de marcas
│   ├── points.rs                                os ficheiros de pontos de um desenho
│   ├── scale.rs                                 de píxeis para milímetros e a distorção radial da objetiva
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: imagem, marcas, escala, objetiva e alinhamento
├── tests/
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   └── vision.rs                                testes da biblioteca e da linha de comandos
├── examples/
│   ├── design.csv                               um exemplo para experimentar o programa
│   ├── marks.pgm                                um exemplo para experimentar o programa
│   ├── measured.csv                             um exemplo para experimentar o programa
│   └── vision.cfg                               o ficheiro de configuração com cada valor e o seu significado
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── rust-toolchain.toml                          a toolchain de Rust fixada
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* A janela ainda não foi vista num ecrã; as marcas são emparelhadas com o desenho pela posição, por isso uma placa muito rodada ou uma marca em falta é recusada em vez de adivinhada.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente.
* Lê um ficheiro de imagem, não uma câmara em direto, só encontra marcas redondas (nem cruzes nem quadrados) e foi experimentado com imagens desenhadas pelos testes, não com fotografias de placas reais.

**A seguir**

* Ler a câmara em direto da máquina (EH3D-NODE-BUNGARD-AUX), encontrar cruzes e quadrados, experimentar fotografias de placas reais e acrescentar à janela o treino de marcas novas.
* Ver a janela num ecrã e terminá-la.
* Experimentá-lo com ficheiros reais e com os outros programas da suíte.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): os crates partilhados, entre eles o leitor de imagens
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): a montagem pick and place
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): a fresagem de isolamento do cobre
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): o programa de controlo de máquina
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): o nó do cabeçote de furação com a câmara
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): o centro que encontra e arranca os programas da suíte

## 📚 Documentação

* [Registo de alterações](CHANGELOG.md)
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
