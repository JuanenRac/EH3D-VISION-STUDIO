<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Fiduciais numa imagem de câmara: marcas, escala e correção de uma placa

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-vision`) com 284 testes que correm num computador: encontra marcas fiduciais redondas numa imagem, converte píxeis em milímetros e calcula a rotação e o deslocamento que põem uma placa onde diz o seu desenho. Tem uma janela (compila-se com `--features gui`, o programa `eh3d-vision-gui`) que abre uma imagem, mostra as marcas com as suas coordenadas, define a escala, ajusta a distorção da objetiva com uma grelha, alinha com o desenho e exporta as opções do ISOCAM, mas a janela ainda não foi vista num ecrã; não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.6` |
| Maturidade | Funcional |
| Testes | 284 |
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
| Testes | 🟡 | 284 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas, e uma câmara simulada sobre uma placa que acrescenta ruído, má exposição, um fotograma morto ou em falta, pó que parece uma marca ou um ficheiro de imagem cortado, e é pedida de novo. |
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
* **Limite de tamanho dos ficheiros que lê:** a linha de comandos só lê uma imagem ou um ficheiro de pontos se não for maior do que `max_file_mib` de `vision.cfg` (64 MiB por predefinição); um ficheiro maior é recusado com o código `file_too_large` e o seu nome, um que não se consegue ler com `file_unreadable` e um ficheiro de pontos que não é texto com `file_not_text`, e não se carrega nada.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.
* **As marcas para o ISOCAM:** `align --isocam-args` escreve cada par de marcas como a opção `--fiducial X_PROJETO,Y_PROJETO,X_MEDIDO,Y_MEDIDO` do `eh3d-isocam`.
* **Uma janela:** compila-se com `--features gui` como o programa `eh3d-vision-gui` (no Windows abre sem consola): abre uma imagem PNG ou PGM com zoom e deslocamento, encontra as marcas com valores editáveis, desenha-as com as suas coordenadas, define a escala com uma marca de tamanho conhecido, ajusta a distorção da objetiva com uma grelha de marcas, alinha com um desenho de um ficheiro CSV (as marcas são emparelhadas com ele pela posição, por isso a ordem não importa) e exporta as opções `--fiducial` do ISOCAM só quando o alinhamento é aceite. Fala os oito idiomas e não precisa de rede.
* **Um ficheiro de configuração:** `vision.cfg` guarda cada valor afinado (o aspeto de uma marca, a grelha do alvo, os limites de um alinhamento aceite, o maior ficheiro lido); um erro nele é recusado com a sua linha e `examples/vision.cfg` enumera-os todos.
* **O centro com fração de píxel e uma confiança:** `find --refine` (ou `refine_subpixel` em `vision.cfg`) pondera os píxeis cinzentos do contorno de cada marca, pelo que o seu centro é encontrado com uma fração de píxel, e escreve uma confiança que desce quando a marca mal se destaca da placa.
* **Luz irregular:** `--flatten PX` (ou `flatten_window`) divide a imagem pela luz à volta de cada píxel antes da procura, para que um candeeiro que esmorece num lado da placa ou uma sombra não escondam as marcas do lado escuro.
* **Marcas em forma de cruz:** `eh3d-vision crosses IMAGEM` encontra sinais de mais com os braços ao longo dos eixos da imagem (caixa quadrada, dois braços da mesma largura, cantos vazios); discos e quadrados são deixados de fora, e `--turned` encontra cruzes com qualquer ângulo (ver abaixo).
* **A escala a partir do passo de um alvo:** `eh3d-vision calibrate IMAGEM --pitch-mm D` toma a mediana da distância entre marcas vizinhas de uma grelha, recusa um alvo que não é visto de frente e escreve a linha `mm_per_px = ...` para o `vision.cfg`.
* **Peças para outros programas, em ficheiros simples:** `eh3d-vision measure IMAGEM --mm-per-px V` dá de cada peça o centro (mm desde o centro da imagem, x para a direita, y para cima), a rotação (de -90 excluído a 90 graus), o comprimento, a largura e a área, em CSV (`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`) ou em JSON (`--format json`: `format` `eh3d-vision-parts`, `version` 1, `unit`, `image_width_px`, `image_height_px`, `mm_per_px`, `parts`); nada mais é partilhado com o programa que os lê. `--output FICHEIRO` nunca escreve por cima de um ficheiro sem `--replace`, nem de uma ligação.
* **Uma pasta de uma vez:** `eh3d-vision batch PASTA --report FICHEIRO.csv` lê cada imagem PNG e PGM dentro do limite de tamanho (no máximo `batch_max_files`) e escreve uma linha por imagem (`file,status,marks,mean_score,min_score,mean_diameter_px,detail`) com o estado `ok`, `no_marks`, `picture_unreadable` ou o código da recusa; um nome de ficheiro que uma folha de cálculo executaria como fórmula é escrito com uma plica à frente.
* **Quadrados e cruzes com qualquer rotação:** `eh3d-vision squares IMAGEM` encontra marcas quadradas rodadas de qualquer ângulo (centro, lado e o ângulo anti-horário como a máquina o vê, de -45 excluído a 45 graus) e `eh3d-vision crosses IMAGEM --turned` encontra cruzes cujos braços não seguem os eixos; o preenchimento mínimo de um quadrado e a pontuação mínima de uma cruz (`square_min_fill`, `cross_min_score`) estão em `vision.cfg`.
* **Marcas em anel de um tamanho ou de dois:** `eh3d-vision rings IMAGEM` encontra marcas redondas com um furo (donuts): cada uma traz o seu centro, o diâmetro exterior e interior, a pontuação da sua forma, uma confiança (a pontuação baixada quando o anel se destaca pouco do fundo, `full_contrast`) e, quando os anéis são de dois tamanhos, se é pequeno ou grande (`--split PX` ou `ring_split` dá o diâmetro que os separa; se não, encontra-se onde os diâmetros caem em dois grupos com um intervalo de pelo menos `ring_size_gap`). Um disco, um anel com uma falha, um anel cortado pela margem, um que não é redondo e um cujo furo é demasiado pequeno ou grande (`ring_min_ratio`, `ring_max_ratio`) ficam de fora. A janela tem uma secção para eles que desenha o círculo exterior e o interior com uma cor para cada tamanho.
* **Onde está seguida uma peça apanhada:** `eh3d-vision offset IMAGEM --package NOME` toma a imagem de uma peça vista de baixo (uma só peça), mede-a como `measure`, compara o seu tamanho com o corpo do seu encapsulamento como `inspect` e dá o seu desvio em relação ao eixo do bico (`--nozzle-x`, `--nozzle-y` ou `nozzle_x_mm`, `nozzle_y_mm`, milímetros desde o centro da imagem, x para a direita e y para cima), a sua rotação em relação à forma como devia estar seguida (`--nominal-angle`; uma peça tão comprida como larga roda pelo caminho mais curto dentro de um quarto de volta, qualquer outra dentro de meia volta) e a correção que a põe onde o bico a quereria: as mesmas distâncias e a rotação ao contrário. CSV ou JSON, nunca escrito sobre um ficheiro sem `--replace`; o código de saída é 1 quando a peça tem o tamanho errado, está seguida mais longe do que `--max-offset` (`pick_max_offset_mm`) ou rodada mais do que `--max-turn` (`pick_max_turn_deg`), depois de o ficheiro ser escrito, e uma imagem sem peça ou com mais de uma peça é recusada (`no_parts`, `parts_count`). É geometria numa imagem: não se usou câmara, bico nem máquina.
* **A lente com dois fatores, recusada se não encaixar:** `eh3d-vision lens IMAGEM` calcula `k1` e `k2` da distorção radial a partir de uma grelha de marcas e escreve as linhas `lens_k1 = ...` e `lens_k2 = ...` para o `vision.cfg`; recusa um ajuste que deixe as marcas a mais de `lens_max_rms` píxeis da sua rede (`lens_residual_too_large`) ou um fator acima de 0,5 (`lens_out_of_range`), e `find --undistort` endireita as posições com eles.
* **Um veredicto para cada peça:** `eh3d-vision inspect IMAGEM --package NOME --table packages.csv` mede as peças de uma imagem e compara cada uma com o corpo do seu encapsulamento (`package`, `body_long_mm`, `body_short_mm` e, se for preciso, `tolerance_mm`, num ficheiro CSV; `examples/packages.csv` traz alguns): `pass` ou `fail` com o motivo (`length`, `width`, `angle`) em CSV ou JSON, e o código de saída 1 quando uma peça falha; a tolerância e a rotação máxima (`inspect_tolerance`, `inspect_max_angle`) estão no `vision.cfg`.
* **Um relatório HTML da pasta:** `eh3d-vision batch PASTA --html relatorio.html` escreve uma página que se basta a si própria (sem scripts, sem ligações, sem outros ficheiros): uma tabela das imagens, um desenho pequeno de cada uma com um círculo em cada marca encontrada e os avisos (uma imagem que não pôde ser usada, sem marcas ou com uma pontuação abaixo de `report_low_score`), na língua de `--lang` ou do `vision.cfg`.
* **As novas ferramentas na janela:** o painel lateral traz o centro com fração de píxel com a confiança de cada marca, a janela da luz, quadrados e cruzes com qualquer rotação (desenhados na imagem), as peças em milímetros (guardadas como `<imagem>-parts.csv`), o relatório da pasta, lido num fio com barra de progresso e botão de cancelar (`eh3d-vision-report.html`, nunca escrito sobre um ficheiro existente sem `replace_output`) e a escala a partir do passo de um padrão, com os textos nas oito línguas e os valores no `vision.cfg`; ainda não foi visto num ecrã.
* **A objetiva na janela, com os dois fatores:** o botão da grelha ajusta `k1` e `k2` como faz `eh3d-vision lens` (recusado acima de `lens_max_rms`, um campo do painel); a janela desenha os lugares onde a objetiva ajustada põe a retícula da grelha, para os comparar com as marcas encontradas (`show_lens_grid`), toma os fatores de `lens_k1` e `lens_k2` do `vision.cfg` sem grelha e guarda-os como `<imagem>-lens.cfg`.
* **Peças face ao seu encapsulamento e imagens de uma peça grande, na janela:** o painel lateral lê a tabela de encapsulamentos (`package_table`), julga as peças medidas face ao encapsulamento `inspect_package` (linha verde ou vermelha por peça, com o motivo da falha; `inspect_tolerance` e `inspect_max_angle` são campos do painel) e guarda os veredictos como `<imagem>-inspection.csv`, o ficheiro de `eh3d-vision inspect`; e planeia as imagens de que precisa uma peça maior do que o campo (o campo é a imagem em milímetros, a peça é `shots_part_width` por `shots_part_height` ou o corpo do encapsulamento, com `shots_overlap` e `shots_margin`), lista para onde vai a câmara e guarda `<imagem>-shots.csv`, o ficheiro de `eh3d-vision shots`. A linha de comandos usa as mesmas duas chaves novas quando não se dá `--package` nem o tamanho da peça.
* **As imagens de uma peça grande, planeadas:** `eh3d-vision shots --package NOME --fov-w MM --fov-h MM` (ou `--body-long` e `--body-short`) lista em CSV as posições da câmara, em milímetros desde o centro da peça, que veem toda uma peça maior do que o campo, com imagens vizinhas sobrepostas em `shots_overlap` e uma margem de `shots_margin`; não precisa de câmara.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Um relatório ou ficheiro de medidas que escreveria por cima de outro ficheiro | só se escreve onde não há nenhum; para escrever por cima é preciso `--replace`, e nunca se escreve através de uma ligação |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_vision_studio` (`use eh3d_vision_studio::...`); a linha de comandos é `eh3d-vision`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision rings IMAGE [--light] [--min-size PX] [--max-size PX] [--split PX] [--flatten PX]   the ring marks found (donuts: a round mark with a hole), with the outer and inner diameter, the size class when the rings are of two sizes (--split gives the diameter that tells small from large, else it is found from the diameters), the score of the shape and a confidence
eh3d-vision offset IMAGE --package NAME [--table FILE] [--mm-per-px V] [--nozzle-x MM] [--nozzle-y MM] [--nominal-angle DEG] [--tolerance MM] [--max-offset MM] [--max-turn DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where a picked part is held, from a picture of it taken from below (one part in it): its offset from the axis of the nozzle and its turn from the way it should be held, the correction that puts it right, its size against the body of the package; exit 1 when it is the wrong size, too far off or turned too much
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuração

Os valores que se afinam vivem num ficheiro simples de `chave = valor`, não no código: `vision.cfg` na pasta onde o programa arranca ou ao lado dele (a variável `EH3D_VISION_CONFIG` indica outro ficheiro). Define o idioma da janela, se as marcas são escuras ou claras, o limiar, o intervalo do diâmetro, o preenchimento e o alongamento de uma marca, o tamanho da marca que define a escala, as colunas e as linhas da grelha, se o alinhamento pode mudar o tamanho, os limites que um alinhamento tem de cumprir e o maior ficheiro lido. `examples/vision.cfg` enumera cada chave com o seu significado e o valor com que o programa arranca; se o ficheiro faltar usam-se esses valores e um ficheiro com um erro é recusado com a sua linha. Na linha de comandos as mesmas coisas dão-se como opções (ver Interfaces). Guarda também os outros valores afinados: a confiança (`refine_subpixel`, `full_contrast`), a janela da luz (`flatten_window`), a escala e o tamanho de uma peça (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), o intervalo de uma cruz (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), a tolerância de um alvo (`pitch_tolerance`) e o máximo de imagens de uma pasta (`batch_max_files`). O preenchimento mínimo de um quadrado e a pontuação mínima de uma cruz com qualquer rotação são `square_min_fill` e `cross_min_score`. O intervalo do lado de um quadrado e quanto a sua caixa pode ser mais comprida do que larga são `square_min_size`, `square_max_size` e `square_max_elongation`, à parte os da cruz. A lente é `lens_k1` e `lens_k2` (como o `lens` os calcula) e o máximo que uma grelha se pode afastar da sua rede depois da correção é `lens_max_rms`. A tabela de encapsulamentos que o `inspect` lê é `package_table`, e o máximo que uma peça se pode afastar do seu encapsulamento e rodar são `inspect_tolerance` e `inspect_max_angle`. A pontuação abaixo da qual o relatório HTML avisa sobre uma imagem é `report_low_score`. A distância entre as marcas do padrão que fixa a escala é `pitch_mm`, e `replace_output` diz se os ficheiros escritos podem ser escritos sobre um que já existe (não por defeito). A sobreposição das imagens vizinhas de um plano de `shots` e a margem em redor da peça são `shots_overlap` e `shots_margin`. Se a janela desenha a retícula da objetiva é `show_lens_grid`. O encapsulamento face ao qual `inspect` e a janela julgam as peças é `inspect_package`, e o tamanho da peça para a qual `shots` e a janela planeiam as imagens é `shots_part_width` e `shots_part_height` (0 e 0: o corpo do encapsulamento). As marcas em anel têm o intervalo do diâmetro exterior (`ring_min_diameter`, `ring_max_diameter`), da relação do diâmetro interior ao exterior (`ring_min_ratio`, `ring_max_ratio`), o preenchimento mínimo da sua banda (`ring_min_fill`), o alongamento (`ring_max_elongation`), o diâmetro que separa os dois tamanhos (`ring_split`, 0 para o encontrar) e o intervalo entre tamanhos (`ring_size_gap`). O eixo do bico na imagem de uma peça apanhada é `nozzle_x_mm` e `nozzle_y_mm`, e o mais longe que pode estar seguida e o mais que pode rodar são `pick_max_offset_mm` e `pick_max_turn_deg` (0: não verificado).

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.88 ou mais recente). O repositório EH3D-COMMON tem de estar clonado ao lado deste: o leitor de imagens é tomado dele por caminho.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Para tirar uma captura da janela sem a abrir à mão, execute-o com `--shot FICHEIRO.png` (também `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`): abre-se com esse tamanho, guarda o PNG, não guarda definições e fecha-se sozinho (código de saída 0 se o ficheiro foi escrito, 1 se não, 2 se os argumentos estiverem errados). Com `--mark N` a marca número N (a partir de 1) define a escala e com `--align` o desenho é alinhado com as marcas, pelo que a captura mostra as linhas do alinhamento: do sítio onde o desenho põe cada marca até à marca encontrada, com a distância em milímetros (`show_residuals` em `vision.cfg` desativa-as). `examples/design_pair.csv` é um desenho para as duas marcas de `examples/marks.pgm`.

## 🧪 Testes

O conjunto tem 284 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

### Transferências

Cada versão traz o programa com a sua janela para Windows (64 e 32 bits), Linux (64 bits) e macOS (Apple silicon e Intel), como `<nome>-<versão>-<sistema>.zip` (Windows) ou `.tar.gz` com o programa, a licença, o leia-me, o ícone e os exemplos. Os ficheiros são construídos pelo GitHub Actions, constam de `SHA284SUMS` e estão assinados com a chave da EH3D. Para verificar uma transferência, compare o seu hash com o ficheiro `.sha284` que a acompanha (`sha284sum -c`) e verifique a assinatura com o ficheiro `.sig` que a acompanha (`ssh-keygen -Y verify`).

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
│   ├── batch.rs                                 uma pasta de imagens de uma vez, com um relatório CSV
│   ├── config.rs                                o ficheiro de configuração: cada valor afinado
│   ├── cross.rs                                 marcas fiduciais em forma de cruz
│   ├── fiducial.rs                              as marcas redondas de uma imagem: limiar, manchas e verificações
│   ├── flatten.rs                               a luz irregular retirada de uma imagem
│   ├── grid.rs                                  a distorção da objetiva a partir de uma grelha de marcas
│   ├── inspect.rs                               cada peça comparada com o corpo do seu encapsulamento a partir de uma tabela, com veredicto
│   ├── job.rs                                   o relatório de uma pasta lido num fio, com progresso e cancelamento
│   ├── lens.rs                                  os dois fatores da lente a partir de uma grelha de marcas, recusados se não encaixarem
│   ├── points.rs                                os ficheiros de pontos de um desenho
│   ├── report.rs                                o relatório de uma pasta como uma página HTML autónoma, em oito línguas
│   ├── ring.rs                                  as marcas em anel: redondas com um furo, de um ou dois tamanhos, com confiança
│   ├── limit.rs                                 leitura de um ficheiro com limite de tamanho e um código para cada recusa
│   ├── measure.rs                               peças medidas em milímetros, ficheiros CSV e JSON
│   ├── offset.rs                                onde está seguida uma peça apanhada, com a correção que a põe bem
│   ├── panels.rs                                os painéis da janela: objetiva, peças face aos encapsulamentos e plano de imagens
│   ├── scale.rs                                 de píxeis para milímetros e a distorção radial da objetiva
│   ├── square.rs                                marcas quadradas e cruzes rodadas de qualquer ângulo
│   ├── tools.rs                                 as ferramentas do painel lateral sobre a imagem: formas, peças, relatório e passo
│   ├── shot.rs                                  o modo de captura da janela (--shot): os seus argumentos, lidos e verificados sem ecrã
│   ├── shots.rs                                 as posições da câmara que cobrem uma peça maior do que o campo, só com os tamanhos
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: imagem, marcas, escala, objetiva e alinhamento
├── tests/
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   ├── read_limit.rs                            testes do limite de tamanho dos ficheiros que a linha de comandos lê
│   ├── exchange.rs                              testes dos comandos que leem uma imagem e escrevem um ficheiro
│   ├── shot_sample.rs                           testes de que os exemplos abrem na janela de uma captura
│   └── vision.rs                                testes da biblioteca e da linha de comandos
├── examples/
│   ├── design.csv                               um exemplo para experimentar o programa
│   ├── design_pair.csv                          um desenho para as duas marcas de marks.pgm
│   ├── marks.pgm                                um exemplo para experimentar o programa
│   ├── measured.csv                             um exemplo para experimentar o programa
│   ├── packages.csv                             uma tabela de corpos de encapsulamentos para o inspect
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
* Lê um ficheiro de imagem, não uma câmara em direto, só encontra marcas redondas, quadradas e em forma de cruz e foi experimentado com imagens desenhadas pelos testes, não com fotografias de placas reais. A rotação de uma peça é medida a partir dos seus píxeis como num retângulo, por isso uma peça que não é um retângulo recebe a rotação do seu eixo longo.

**A seguir**

* Ler a câmara em direto da máquina (EH3D-NODE-BUNGARD-AUX), experimentar fotografias de placas reais e acrescentar à janela o treino de marcas novas.
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
