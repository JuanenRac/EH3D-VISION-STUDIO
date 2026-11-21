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
  <img src="https://img.shields.io/badge/Version-0.1.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-vision`) com 687 testes que correm num computador: encontra marcas fiduciais redondas numa imagem, converte píxeis em milímetros e calcula a rotação e o deslocamento que põem uma placa onde diz o seu desenho. Tem uma janela (compila-se com `--features gui`, o programa `eh3d-vision-gui`) que abre uma imagem, mostra as marcas com as suas coordenadas, define a escala, ajusta a distorção da objetiva com uma grelha, alinha com o desenho e exporta as opções do ISOCAM, mas a janela ainda não foi vista num ecrã; não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.1.2` |
| Maturidade | Funcional |
| Testes | 687 |
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
| Testes | 🟡 | 687 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas, e uma câmara simulada sobre uma placa que acrescenta ruído, má exposição, um fotograma morto ou em falta, pó que parece uma marca ou um ficheiro de imagem cortado, e é pedida de novo. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`; uma pequena fronteira unsafe só nos motores de câmara compilados com a sua funcionalidade): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🟠 | Escrita (egui, `--features gui`, o programa `eh3d-vision-gui`): abrir ou largar uma imagem, ver as marcas com as suas coordenadas, definir a escala com uma marca de tamanho conhecido, ajustar a distorção da objetiva com uma grelha, alinhar com um desenho CSV e exportar as opções do ISOCAM; a sua lógica está testada, mas ainda não foi compilada nem vista num ecrã. |
| Câmara em direto | 🟠 | a camada da câmara, o tabuleiro de calibração e o alinhamento de uma placa estão escritos e testados com uma câmara e uma mesa simuladas; os motores do Windows, do V4L2 e da libcamera são esqueletos e nada correu numa câmara, lente ou placa reais. |
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

## 🖼️ Capturas de ecrã

<p align="center">
  <img src="images/screens/vision-studio-01-window.png" alt="A janela com a imagem e o desenho de exemplo" width="100%">
</p>

<p align="center"><em>A janela com a imagem e o desenho de exemplo: as duas marcas encontradas, cada uma com o seu centro e o seu número, e as definições da pesquisa à esquerda.</em></p>

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `eh3d-vision` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Linha de comandos estrita:** uma opção mal escrita (`--powr`) ou uma palavra a mais é um erro que a nomeia, não algo que se ignora; caso contrário o programa faria a sua saída com os valores por omissão.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Limite de tamanho dos ficheiros que lê:** a linha de comandos só lê uma imagem ou um ficheiro de pontos se não for maior do que `max_file_mib` de `vision.cfg` (64 MiB por predefinição); um ficheiro maior é recusado com o código `file_too_large` e o seu nome, um que não se consegue ler com `file_unreadable` e um ficheiro de pontos que não é texto com `file_not_text`, e não se carrega nada.
* **Sem código unsafe na compilação por omissão:** `#![forbid(unsafe_code)]` na biblioteca e no programa; só os motores das câmaras em direto, compilados com a sua própria funcionalidade, têm uma pequena fronteira unsafe (poucas chamadas, com os tamanhos das estruturas verificados ao compilar).
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
* **Uma tabela de correções para um tabuleiro e um painel da janela para ela:** `eh3d-vision offset IMAGEM --package NOME --each` toma cada peça da imagem (um tabuleiro), cada uma por si, e escreve uma tabela com a coluna `part` (o número da peça, a partir de 1) e depois as colunas de `offset`, uma linha por peça; o código de saída é 1 quando alguma peça falha. A janela tem um painel para o mesmo (`nozzle_x_mm`, `nozzle_y_mm`, `nominal_angle_deg`, `pick_max_offset_mm`, `pick_max_turn_deg`): calcula a tabela da imagem aberta, lista a correção de cada peça no idioma da janela e guarda-a como `<imagem>-offsets.csv`. `nominal_angle_deg` é a rotação de uma peça bem presa, também em `vision.cfg`.
* **Uma câmara em espelho e o eixo do bico:** `--mirror` (`pick_mirror`) serve para uma câmara que vê as peças viradas da esquerda para a direita: o x e a rotação de cada peça mudam de sinal. `eh3d-vision nozzle IMAGEM IMAGEM... --package NOME` toma imagens de peças apanhadas com o mesmo bico (uma peça em cada, pelo menos duas imagens) e escreve as linhas `nozzle_x_mm` e `nozzle_y_mm` para `vision.cfg`, o lugar médio das peças (não depende do eixo que estava em uso quando foram medidas); na saída de erros diz quanto as peças se dispersam (o desvio-padrão em x e em y, a peça mais afastada e a rotação média).
* **Se a imagem chega para medir:** `eh3d-vision quality IMAGEM` dá a nitidez (a variância do laplaciano), o contraste (o desvio-padrão dos níveis de cinzento) e o nível de cinzento médio de uma imagem, em CSV ou JSON, e recusa-a (código de saída 1, depois de escrito o ficheiro) como `flat`, `dark`, `bright` ou, quando `--min-sharpness` é maior que 0, `blurry`; os mínimos são `--min-contrast`, `--min-mean`, `--max-mean` e `--min-sharpness`, ou as chaves de `vision.cfg`. Com `quality_check = yes`, `offset`, `nozzle` e a janela recusam uma imagem fraca (`poor_picture`) antes de nela medirem uma peça, e a janela tem um painel para verificar a imagem e mostrar os seus números. Os valores dependem da câmara e da objetiva, por isso estão no ficheiro de configuração e a nitidez começa desativada; nada disto foi experimentado com uma câmara real.
* **A lente com dois fatores, recusada se não encaixar:** `eh3d-vision lens IMAGEM` calcula `k1` e `k2` da distorção radial a partir de uma grelha de marcas e escreve as linhas `lens_k1 = ...` e `lens_k2 = ...` para o `vision.cfg`; recusa um ajuste que deixe as marcas a mais de `lens_max_rms` píxeis da sua rede (`lens_residual_too_large`) ou um fator acima de 0,5 (`lens_out_of_range`), e `find --undistort` endireita as posições com eles.
* **Um veredicto para cada peça:** `eh3d-vision inspect IMAGEM --package NOME --table packages.csv` mede as peças de uma imagem e compara cada uma com o corpo do seu encapsulamento (`package`, `body_long_mm`, `body_short_mm` e, se for preciso, `tolerance_mm`, num ficheiro CSV; `examples/packages.csv` traz alguns): `pass` ou `fail` com o motivo (`length`, `width`, `angle`) em CSV ou JSON, e o código de saída 1 quando uma peça falha; a tolerância e a rotação máxima (`inspect_tolerance`, `inspect_max_angle`) estão no `vision.cfg`.
* **Um relatório HTML da pasta:** `eh3d-vision batch PASTA --html relatorio.html` escreve uma página que se basta a si própria (sem scripts, sem ligações, sem outros ficheiros): uma tabela das imagens, um desenho pequeno de cada uma com um círculo em cada marca encontrada e os avisos (uma imagem que não pôde ser usada, sem marcas ou com uma pontuação abaixo de `report_low_score`), na língua de `--lang` ou do `vision.cfg`.
* **As novas ferramentas na janela:** o painel lateral traz o centro com fração de píxel com a confiança de cada marca, a janela da luz, quadrados e cruzes com qualquer rotação (desenhados na imagem), as peças em milímetros (guardadas como `<imagem>-parts.csv`), o relatório da pasta, lido num fio com barra de progresso e botão de cancelar (`eh3d-vision-report.html`, nunca escrito sobre um ficheiro existente sem `replace_output`) e a escala a partir do passo de um padrão, com os textos nas oito línguas e os valores no `vision.cfg`; ainda não foi visto num ecrã.
* **A objetiva na janela, com os dois fatores:** o botão da grelha ajusta `k1` e `k2` como faz `eh3d-vision lens` (recusado acima de `lens_max_rms`, um campo do painel); a janela desenha os lugares onde a objetiva ajustada põe a retícula da grelha, para os comparar com as marcas encontradas (`show_lens_grid`), toma os fatores de `lens_k1` e `lens_k2` do `vision.cfg` sem grelha e guarda-os como `<imagem>-lens.cfg`.
* **Peças face ao seu encapsulamento e imagens de uma peça grande, na janela:** o painel lateral lê a tabela de encapsulamentos (`package_table`), julga as peças medidas face ao encapsulamento `inspect_package` (linha verde ou vermelha por peça, com o motivo da falha; `inspect_tolerance` e `inspect_max_angle` são campos do painel) e guarda os veredictos como `<imagem>-inspection.csv`, o ficheiro de `eh3d-vision inspect`; e planeia as imagens de que precisa uma peça maior do que o campo (o campo é a imagem em milímetros, a peça é `shots_part_width` por `shots_part_height` ou o corpo do encapsulamento, com `shots_overlap` e `shots_margin`), lista para onde vai a câmara e guarda `<imagem>-shots.csv`, o ficheiro de `eh3d-vision shots`. A linha de comandos usa as mesmas duas chaves novas quando não se dá `--package` nem o tamanho da peça.
* **As imagens de uma peça grande, planeadas:** `eh3d-vision shots --package NOME --fov-w MM --fov-h MM` (ou `--body-long` e `--body-short`) lista em CSV as posições da câmara, em milímetros desde o centro da peça, que veem toda uma peça maior do que o campo, com imagens vizinhas sobrepostas em `shots_overlap` e uma margem de `shots_margin`; não precisa de câmara.
* **Marcas por correlação:** `eh3d-vision correlate IMAGEM --diameter PX` desliza um disco desse diâmetro sobre a imagem e calcula em cada ponto a correlação normalizada (não muda se a luz for mais forte ou o contraste menor), pelo que se encontra uma marca que o limiar perde com luz irregular; os pontos são dados com fração de píxel e a pontuação, o melhor primeiro, nenhum mais perto de um melhor do que um diâmetro. Precisa do diâmetro (`correlate_diameter`, `correlate_min_score`) e é mais lenta do que a pesquisa por limiar, pelo que uma imagem que demoraria demasiado é recusada (`correlate_too_big`).
* **Perspetiva corrigida:** `eh3d-vision rectify IMAGEM --corners X1,Y1,...,X4,Y4 --output FICHEIRO.pgm` desenha de novo a imagem como seria vista de cima, a partir dos quatro cantos de um retângulo nela (uma homografia com amostragem bilinear); cantos que não formam um quadrilátero convexo são recusados (`bad_corners`) e o lado maior é `rectify_max_side`.
* **Uma fila de pads:** `eh3d-vision pads IMAGEM --mm-per-px V` toma as peças da imagem como pads e dá o passo (a distância mediana ao longo da reta de melhor ajuste), quanto varia, quanto um pad se afasta da fila e o seu tamanho médio, em CSV ou JSON; o veredicto é `fail` com os motivos `pitch`, `row` e `expected` (`--expect-pitch`), conforme `pads_tolerance`, e o código de saída é 1.
* **Antes e depois:** `eh3d-vision diff ANTES DEPOIS` compara duas imagens do mesmo tamanho: os píxeis cujo nível de cinzento difere em mais de `diff_threshold`, a sua proporção, a diferença média e a maior, a caixa que os rodeia e as regiões de pelo menos `diff_min_area` píxeis; `--mask FICHEIRO.pgm` escreve a imagem das alterações e o código de saída é 1 quando mudou mais de `diff_max_changed`.
* **Em que sentido está uma peça:** `eh3d-vision orientation IMAGEM --mm-per-px V` encontra a peça (o que difere da placa) e a marca que ela tem (a banda de um díodo, o ponto do pino 1: o que difere do corpo dentro dela, mesmo que se pareça com a placa) e dá a direção do meio da peça até à marca (graus no sentido anti-horário a partir do eixo x, y para cima), o quarto de volta mais próximo e quanto se afasta dele e, com `--expect-deg`, quanto se afasta da direção desejada. A peça é recusada (código de saída 1, depois de escrever o ficheiro) como `no_mark`, `centred`, `mark_big`, `wrong_way` ou, sem direção desejada, `skewed`.
* **A pegada dos pads:** `eh3d-vision footprint IMAGEM --pad-length MM --pad-width MM` mede cada pad e julga cada um por si contra o desenho com uma tolerância em milímetros (`--tolerance`, `footprint_tolerance_mm`): uma linha por pad, da fila de cima para baixo e da esquerda para a direita, com as diferenças e os motivos `length` e `width`; `--count` verifica também o número de pads (`count`) e o código de saída é 1 quando um pad está fora.
* **Um mapa de onde a imagem é nítida:** `eh3d-vision sharpmap IMAGEM --cells N` corta a imagem numa grelha e dá a nitidez de cada célula (a variância do laplaciano), a mediana, a célula mais fraca, a inclinação da nitidez ao longo de x e de y e as células fracas (abaixo de `--min-share` da mediana); uma célula quase sem contraste está `empty` e não é julgada. `--heat FICHEIRO.pgm` escreve a imagem do mapa, escura onde a imagem está desfocada, e o código de saída é 1 quando uma célula é fraca (`uneven`).
* **A escala a partir de um tabuleiro de quadrados, com relatório:** `eh3d-vision board IMAGEM --pitch-mm D --square-mm S` encontra os quadrados, calcula a rotação do tabuleiro e a distância entre vizinhos ao longo de cada eixo, e dá os milímetros de um píxel ao longo de x, de y e a partir do lado, com o quão quadrados são os píxeis (`aspect`) e quanto diferem as distâncias (`spread`); `--report FICHEIRO.txt` escreve o relato para uma pessoa com a linha `mm_per_px = ...` para `vision.cfg`. É recusada (código de saída 1) quando os píxeis não são quadrados (`aspect`), as distâncias diferem (`spread`) ou o lado e o passo não concordam (`disagree`).
* **Mais quatro painéis na janela:** o painel lateral tem a procura por correlação (as marcas encontradas são listadas com o lugar e a pontuação), a imagem endireitada com as quatro marcas encontradas (guardada como `<imagem>-rectified.pgm`), a fila de pads julgada pelas peças medidas (guardada como `<imagem>-pads.csv`) e a comparação da imagem aberta com outra (guardada como `<imagem>-diff.csv` e `<imagem>-changes.pgm`); os seus valores são campos dos painéis (`correlate_diameter`, `correlate_min_score`, `rectify_max_side`, `pads_tolerance`, `diff_threshold`, `diff_min_area`, `diff_max_changed`) e os seus textos estão nos oito idiomas. A lógica está testada; a janela não foi vista num ecrã.
* **O servidor da máquina:** `eh3d-vision server health|config|discover|check|detect|send` usa as mesmas chaves dos outros programas da suite (`server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns`) através do cliente partilhado da suite (`eh3d-serverclient` do EH3D-COMMON): `config` lê os valores de `vision.cfg` que o servidor guarda para a máquina (o artefacto mais recente `vision-<machine>.cfg` do formato `text/x-eh3d-vision-config`, escrito como as linhas do ficheiro ou como um objeto JSON; tudo ou nada; as chaves da ligação nunca são tiradas do servidor) e escreve-os como linhas para o ficheiro, e `send quality|offset|pads|diff|orientation|footprint|sharpmap|board` guarda no servidor o resultado JSON desse comando como o artefacto `vision-<kind>.json` (`POST /api/v1/artifacts`, com a máquina e o tipo nos parâmetros). As novas tentativas e o estado da ligação são os do cliente partilhado (um pedido que pode ter chegado nunca é enviado duas vezes), as recusas do servidor podem ser ditas pelo seu código em oito idiomas, o token nunca é impresso e avisa-se de um token que iria em HTTP simples para um endereço que não é local. O servidor tem um plugin `vision`, por isso os resultados também se guardam na sua convenção (os pontos seguintes); testa-se contra um servidor em memória (`server::SimServer`) e contra um socket no loopback, não contra o servidor real.
* **Resultados na convenção do servidor:** `eh3d-vision server send fiducials|focus|blobs IMAGEM` guarda um resultado na forma que partilham o plugin `vision` do EH3D-SERVER e os programas que olham através de uma câmara: um ficheiro `vision-<tipo>.json` de formato `application/json` e esquema `eh3d.vision.result/1`, com o tamanho e o SHA-256 da imagem, os achados do tipo (`fiducials` `{x, y, radius_px, score}`, `focus` `{score, sharp}`, `blobs` `{x, y, area_px}`; `barcode` é verificado mas nunca feito, porque aqui nada lê códigos), as unidades (`px`) e a evidência (`picture`, salvo se `--evidence camera|simulator` disser outra coisa: o programa não pode saber que um ficheiro veio de uma câmara). `server check FICHEIRO.json` diz se um documento segue a convenção e nomeia a parte que não a segue, e um documento que não a segue é recusado antes de ser enviado. A pontuação de uma verificação de foco é 50 quando a nitidez é `focus_reference`.
* **A estação simulada do servidor:** `eh3d-vision server detect fiducials|barcode|focus|blobs IMAGEM [--seed N] [--artifact]` pede ao plugin `vision` do servidor (`POST /api/v1/work/jobs`, iniciado e lido até terminar) o resultado que ele inventa a partir do tamanho e do hash da imagem e da semente, sempre o mesmo (`evidence: simulator`), na vista do trabalho ou guardado como artefacto. `station::simulate` é uma portagem do plugin com que responde o servidor em memória, de modo que toda a cadeia de trabalho, resultado e artefacto é testada aqui sem rede; não foi executada contra o servidor real.
* **A procura do servidor na rede, como interruptor real:** `eh3d-vision server discover` lista o `anfitrião:porta` dos servidores que respondem a `_eh3d._tcp` na rede local. Não faz nada enquanto `server_mdns` estiver desligado (por omissão), e o programa tem de ser compilado com `--features mdns` (os das versões estão) para ter o resolvedor do cliente partilhado. O que encontra são candidatos para a pessoa escolher, nunca ligados por si, porque qualquer dispositivo da rede pode responder.
* **Uma aresta com fração de píxel e a sua incerteza:** `eh3d-vision edge IMAGEM --from X,Y --to X,Y` lê a imagem ao longo de uma linha traçada através de uma aresta (bilinear, a `edge_step_px` de distância), toma as duas pontas da linha como os dois fundos planos, encontra onde o perfil cruza o nível a meio caminho e dá a posição ao longo da linha e na imagem, o contraste, a largura da transição e a incerteza (um desvio-padrão: o ruído dos fundos, com o arredondamento de 8 bits somado, sobre a inclinação onde cruza). O código de saída é 1 quando a incerteza passa `edge_max_uncertainty_px`; uma linha sem aresta é `edge_flat`. Uma aresta mais nítida do que um píxel pode desviar-se até 0,05 px.
* **O histograma e o limiar automático:** `eh3d-vision histogram IMAGEM --bins N` dá os níveis de cinzento em 8 a 256 grupos e os cortes de Otsu (o que a procura de marcas usa), de isodata e do método do triângulo, quanta variância os dois grupos de Otsu explicam, a parte de píxeis pretos e brancos e se os três cortes coincidem (dentro de 8 níveis); se não coincidem, convém ver a iluminação ou o `threshold` de `vision.cfg`.
* **Uma imagem contra uma referência, através de uma máscara de tolerância:** `eh3d-vision refcheck REFERÊNCIA IMAGEM --tolerance MÁSCARA.pgm` compara duas imagens do mesmo tamanho onde a diferença permitida pode mudar de sítio para sítio: um píxel da máscara a 0 mantém o `diff_threshold` geral, de 1 a 254 permite esses níveis de cinzento a mais, e 255 não é olhado. Dá os píxeis fora da tolerância, quanto a ultrapassam, em quantas regiões (`diff_min_area`) e onde; `--map FICHEIRO.pgm` escreve-os como imagem e o código de saída é 1 acima de `diff_max_changed`.
* **Os resíduos da lente, marca a marca:** `eh3d-vision lens IMAGEM --residuals` escreve, para cada marca da grelha, quanto se afasta do seu lugar antes e depois da correção, a marca mais afastada, as três bandas de distância ao centro e a razão da banda exterior para a interior: uma razão bem acima de 1 diz que dois termos radiais não chegam para a lente (ou que o alvo não é plano).
* **Mais na janela:** o painel lateral tem as definições do servidor (o painel da suite do `eh3d-view`: endereço, token, máquina, novas tentativas, o interruptor da procura na rede, início de sessão e sessões), o estado da ligação no canto, a procura de servidores (só com o interruptor ligado, numa thread, com candidatos para escolher), um histograma com os três cortes, a aresta ao longo de uma linha de quatro números, a verificação contra uma referência com máscara de tolerância e uma linha de ajuda fechada sob os títulos dos painéis, nos oito idiomas. A janela não foi vista num ecrã.
* **Testes com semente fixa:** `tests/fuzz.rs` dá imagens de qualquer tamanho, cantos em qualquer sítio, pads de qualquer tamanho, quadrados em qualquer lugar e ficheiros de configuração, JSON, Base64 e respostas HTTP danificados, a partir de um gerador que não lê o relógio; nada pode entrar em pânico e o que sai tem de fazer sentido, e os painéis da janela passam da mesma forma por imagens aleatórias. `tests/fuzz_camera.rs` faz o mesmo com uma aresta ao longo de qualquer linha de qualquer imagem, o histograma, uma imagem contra uma referência com qualquer máscara, os resíduos de marcas quaisquer e documentos da convenção cortados, alterados ou cheios de lixo.
* **Uma câmara em direto atrás de um trait, experimentada só com uma simulada (`src/capture/`):** as câmaras são listadas pelo nome com uma chave que não muda (`vendor:product:serial@port`), abertas num formato (YUYV, JPEG em movimento, RGB, cinzento) e dão imagens com um contador e os seus bytes originais; cada uma das 13 propriedades tem o seu intervalo ou a câmara diz que não a tem. Encontram-se a carga de um barramento USB (`capture_usb_mbps`), as imagens pretas, repetidas, perdidas ou cortadas e a cintilação da luz, esperam-se imagens novas pela contagem e não por segundos, e o registo vai para uma função. A câmara simulada falha de propósito; os motores do Windows, do V4L2 e da libcamera são esqueletos atrás das funcionalidades `mfoundation`, `v4l2` e `libcamera` (desligadas por omissão) e não foram experimentados numa câmara real, e as imagens de JPEG em movimento são verificadas mas ainda não descodificadas.
* **Um tabuleiro de calibração com marcadores numerados (`src/charuco/`):** `eh3d-vision charuco card` desenha um tabuleiro de xadrez cujas casas brancas levam marcadores, em milímetros para imprimir a 100 %; `eh3d-vision charuco calibrate IMAGE` lê os marcadores, encontra os cantos com uma fração de píxel com qualquer rotação e ajusta o mapa de píxeis para milímetros de três maneiras (homografia, afim, semelhança). É recusada se deixa demasiado erro nos cantos (`calib_max_residual_px`), se os métodos afastam demasiado os cantos (`calib_max_method_gap`) ou se não coincide com a distância medida com uma régua entre os dois cantos exteriores de uma fila (`calib_check_mm`); sem régua não se usa, a menos que `calib_allow_unchecked` o permita. O único desvio é o de `camera_offset_x_mm`, `camera_offset_y_mm` e `camera_offset_deg`, e uma calibração de imagens simuladas é recusada para uma câmara real.
* **Uma placa posta no lugar a ver, mover e voltar a ver (`src/boardaim.rs`):** os fiduciais são vistos a partir de uma pose que não muda, calculam-se a rotação e o desvio e manda-se a mesa mover a placa no sentido contrário (nunca mais de `align_max_step_mm` e `align_max_step_deg` de cada vez, nem `align_max_total_mm` no total); depois de esperar imagens tiradas após o movimento volta a ver, até a placa estar dentro de `align_tolerance_mm` e `align_tolerance_deg`. Quando algo corre mal só se vai para o estado mais seguro (passos inteiros, meios passos, reter, abortar): um erro que cresce reduz o passo a metade e depois pára, e a falta de marcas, imagens pretas ou repetidas, um limite ou uma falha da mesa param os movimentos. Só foi executada com uma mesa simulada.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Um relatório ou ficheiro de medidas que escreveria por cima de outro ficheiro | só se escreve onde não há nenhum; para escrever por cima é preciso `--replace`, e nunca se escreve através de uma ligação |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)`; uma pequena fronteira unsafe só nos motores de câmara compilados com a sua funcionalidade |
| Um token mostrado ou enviado onde não deve | o token nunca é impresso nem posto num resultado; os valores que um servidor envia são tomados todos ou nenhum e as chaves da ligação (`server_*`) nunca são tiradas dele; avisa-se de um token que iria em HTTP simples para um endereço que não é local |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_vision_studio` (`use eh3d_vision_studio::...`); a linha de comandos é `eh3d-vision`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Só o comando `server` usa a rede (HTTP simples para o servidor da máquina indicado em `vision.cfg`); todos os outros trabalham com ficheiros locais.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX] [--residuals] [--format csv|json] [--output FILE] [--replace]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice; --residuals writes, mark by mark, how far each lies from its place before and after the correction, the farthest mark, three bands of distance from the centre and the ratio of the outer band to the inner one (a ratio well above 1: two radial terms are not enough)
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision rings IMAGE [--light] [--min-size PX] [--max-size PX] [--split PX] [--flatten PX]   the ring marks found (donuts: a round mark with a hole), with the outer and inner diameter, the size class when the rings are of two sizes (--split gives the diameter that tells small from large, else it is found from the diameters), the score of the shape and a confidence
eh3d-vision offset IMAGE --package NAME [--table FILE] [--mm-per-px V] [--nozzle-x MM] [--nozzle-y MM] [--nominal-angle DEG] [--tolerance MM] [--max-offset MM] [--max-turn DEG] [--mirror] [--each] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where a picked part is held, from a picture of it taken from below (one part in it): its offset from the axis of the nozzle and its turn from the way it should be held, the correction that puts it right, its size against the body of the package; --mirror when the camera sees the part turned over left to right; --each takes every part of the picture (a tray) and writes a table of corrections, one row for each; exit 1 when a part is the wrong size, too far off or turned too much
eh3d-vision nozzle IMAGE IMAGE... --package NAME [--table FILE] [--mm-per-px V] [--mirror] [--light] [--flatten PX] [--output FILE] [--replace]   where the axis of the nozzle is, from pictures of parts picked with it (one part in each, at least two pictures): the lines nozzle_x_mm and nozzle_y_mm for vision.cfg, with how spread the parts are
eh3d-vision quality IMAGE [--min-contrast V] [--min-mean V] [--max-mean V] [--min-sharpness V] [--format csv|json] [--output FILE] [--replace]   whether a picture is good enough to measure from: its sharpness, contrast and mean grey level, and why it is refused (flat, dark, bright or blurry); exit 1 when it is refused
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision correlate IMAGE [--diameter PX] [--min-score S] [--light] [--flatten PX]   the round marks found by sliding a disc of that diameter over the picture and working out the correlation at every place (a mark that the search by threshold loses on uneven light), with the place to a fraction of a pixel and the score
eh3d-vision rectify IMAGE --corners X1,Y1,X2,Y2,X3,Y3,X4,Y4 --output FILE.pgm [--size WxH] [--fill V] [--replace]   the picture drawn again as seen from straight above, from the four corners of a rectangle in it (top left, top right, bottom right, bottom left, in pixels)
eh3d-vision pads IMAGE [--mm-per-px V] [--expect-pitch MM] [--tolerance F] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   a row of pads: the pitch, how much it varies, how far the pads lie from the row and their size; exit 1 when the pitch varies, a pad is off the row or the pitch is not the one asked for
eh3d-vision diff BEFORE AFTER [--threshold V] [--min-area PX] [--max-changed F] [--mask FILE.pgm] [--format csv|json] [--output FILE] [--replace]   two pictures of the same size compared: how many pixels changed, how much, where and in how many regions; exit 1 when more than --max-changed of the pixels changed
eh3d-vision orientation IMAGE [--expect-deg D] [--tolerance DEG] [--body-contrast V] [--mark-contrast V] [--min-mark-px N] [--mm-per-px V] [--format csv|json] [--output FILE] [--replace]   which way a part with a mark on it (the band of a diode, the dot of pin 1) is held: the direction from the middle of the part to the mark, the quarter of a turn it is nearest to and how far from it, and how far from the wanted direction; exit 1 when there is no mark, it is in the middle, or it points off the wanted direction (or off a quarter of a turn when none is asked for)
eh3d-vision footprint IMAGE --pad-length MM --pad-width MM [--tolerance MM] [--count N] [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each pad of a picture judged on its own against the size of the design, with a tolerance in millimetres: one line per pad with the differences; exit 1 when a pad is off or the count is not the one asked for
eh3d-vision sharpmap IMAGE [--cells N] [--min-share F] [--heat FILE.pgm] [--format csv|json] [--output FILE] [--replace]   a map of where the picture is sharp: a grid of cells with the sharpness of each, the weak ones, the tilt along x and y and (with --heat) a picture of the map; exit 1 when a cell is much less sharp than the median
eh3d-vision board IMAGE [--square-mm D] [--pitch-mm D] [--max-aspect F] [--max-spread F] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--report FILE.txt] [--replace]   the millimetres of a pixel from a board of squares (D is the side or the distance between centres), along x, along y and from the side, with a report; exit 1 when the pixels are not square, the distances between neighbours differ or the side and the pitch do not agree
eh3d-vision edge IMAGE --from X,Y --to X,Y [--step PX] [--max-uncertainty PX] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where an edge is, to a fraction of a pixel, along a line laid across it (from a point on one side to a point on the other, both inside the picture), with the uncertainty of the position (one standard deviation, from the noise of the two flat grounds and the slope of the edge), the contrast and the width of the transition; exit 1 when the uncertainty is above the limit
eh3d-vision histogram IMAGE [--bins N] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the histogram of the grey levels in 8 to 256 groups and the thresholds that Otsu, isodata and the triangle method pick, how much of the variance two groups explain and whether the three agree (the value for `threshold` of vision.cfg when the automatic one is not good)
eh3d-vision refcheck REFERENCE PICTURE [--tolerance MASK.pgm] [--threshold V] [--min-area PX] [--max-changed F] [--map FILE.pgm] [--format csv|json] [--output FILE] [--replace]   a picture compared with a reference of the same size through a mask of tolerance (a mask pixel at 0 keeps the general threshold, 1 to 254 allows that many levels more, 255 is not looked at): the pixels outside the tolerance, how much they pass it, in how many regions and where; --map writes them as a picture; exit 1 when the share is above the limit
eh3d-vision charuco card [--lang CODE] [--output FILE.svg] [--replace]   the board of calibration (squares and markers, calib_squares_x, calib_squares_y, calib_square_mm and calib_marker_seed of vision.cfg) as a drawing in millimetres to print at 100 %, with the guide in the language
eh3d-vision charuco calibrate IMAGE [--check-mm D] [--allow-unchecked] [--output FILE.cal] [--replace]   the calibration of the camera from a picture of that board: the markers are read, the corners found, the map from pixels to millimetres fitted three ways and refused when the residual, the gap between the methods or the distance D measured with a ruler between the two outer corners of a row (calib_check_mm) do not hold; without D the calibration is refused unless --allow-unchecked or calib_allow_unchecked
eh3d-vision server health|config|discover|check|detect|send KIND ... [--address HOST:PORT] [--machine ID]   the EH3D server of the machine (server_address, server_token, server_machine and server_mdns of vision.cfg): health asks whether it answers, config reads the values of the configuration file that the server keeps for the machine (written as lines for vision.cfg with --output, never over a file without --replace), discover lists the servers that answer on the local network (only with server_mdns = on and a program built with the feature mdns; they are candidates for the person to choose, never connected to by themselves), check FILE tells whether a result follows the convention of the server (schema, kind, picture, findings, units, evidence), detect fiducials|barcode|focus|blobs IMAGE [--seed N] [--artifact] asks the simulated station of the server for a result about the picture (evidence simulator), send quality|offset|pads|diff|orientation|footprint|sharpmap|board takes the arguments of the command of that name and keeps its result as JSON on the server, and send fiducials|focus|blobs IMAGE [--evidence camera|picture|simulator] [--max-findings N] keeps the result in the convention of the server with its schema
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuração

Os valores que se afinam vivem num ficheiro simples de `chave = valor`, não no código: `vision.cfg` na pasta onde o programa arranca ou ao lado dele (a variável `EH3D_VISION_CONFIG` indica outro ficheiro). Define o idioma da janela, se as marcas são escuras ou claras, o limiar, o intervalo do diâmetro, o preenchimento e o alongamento de uma marca, o tamanho da marca que define a escala, as colunas e as linhas da grelha, se o alinhamento pode mudar o tamanho, os limites que um alinhamento tem de cumprir e o maior ficheiro lido. `examples/vision.cfg` enumera cada chave com o seu significado e o valor com que o programa arranca; se o ficheiro faltar usam-se esses valores e um ficheiro com um erro é recusado com a sua linha. Na linha de comandos as mesmas coisas dão-se como opções (ver Interfaces). Guarda também os outros valores afinados: a confiança (`refine_subpixel`, `full_contrast`), a janela da luz (`flatten_window`), a escala e o tamanho de uma peça (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), o intervalo de uma cruz (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), a tolerância de um alvo (`pitch_tolerance`) e o máximo de imagens de uma pasta (`batch_max_files`). O preenchimento mínimo de um quadrado e a pontuação mínima de uma cruz com qualquer rotação são `square_min_fill` e `cross_min_score`. O intervalo do lado de um quadrado e quanto a sua caixa pode ser mais comprida do que larga são `square_min_size`, `square_max_size` e `square_max_elongation`, à parte os da cruz. A lente é `lens_k1` e `lens_k2` (como o `lens` os calcula) e o máximo que uma grelha se pode afastar da sua rede depois da correção é `lens_max_rms`. A tabela de encapsulamentos que o `inspect` lê é `package_table`, e o máximo que uma peça se pode afastar do seu encapsulamento e rodar são `inspect_tolerance` e `inspect_max_angle`. A pontuação abaixo da qual o relatório HTML avisa sobre uma imagem é `report_low_score`. A distância entre as marcas do padrão que fixa a escala é `pitch_mm`, e `replace_output` diz se os ficheiros escritos podem ser escritos sobre um que já existe (não por defeito). A sobreposição das imagens vizinhas de um plano de `shots` e a margem em redor da peça são `shots_overlap` e `shots_margin`. Se a janela desenha a retícula da objetiva é `show_lens_grid`. O encapsulamento face ao qual `inspect` e a janela julgam as peças é `inspect_package`, e o tamanho da peça para a qual `shots` e a janela planeiam as imagens é `shots_part_width` e `shots_part_height` (0 e 0: o corpo do encapsulamento). As marcas em anel têm o intervalo do diâmetro exterior (`ring_min_diameter`, `ring_max_diameter`), da relação do diâmetro interior ao exterior (`ring_min_ratio`, `ring_max_ratio`), o preenchimento mínimo da sua banda (`ring_min_fill`), o alongamento (`ring_max_elongation`), o diâmetro que separa os dois tamanhos (`ring_split`, 0 para o encontrar) e o intervalo entre tamanhos (`ring_size_gap`). O eixo do bico na imagem de uma peça apanhada é `nozzle_x_mm` e `nozzle_y_mm`, e o mais longe que pode estar seguida e o mais que pode rodar são `pick_max_offset_mm` e `pick_max_turn_deg` (0: não verificado). A rotação de uma peça bem presa é `nominal_angle_deg`, se a câmara vê as peças em espelho é `pick_mirror`, e a verificação da imagem é `quality_check` com os mínimos `quality_min_contrast`, `quality_min_mean`, `quality_max_mean` e `quality_min_sharpness` (0: não verificada). O servidor é `server_address`, `server_token`, `server_machine`, `server_timeout_s` e `server_retries` (endereço vazio: não há servidor); a pesquisa por correlação é `correlate_diameter` e `correlate_min_score`; o lado maior de uma imagem corrigida é `rectify_max_side`; a tolerância de uma fila de pads é `pads_tolerance`; e a comparação de duas imagens é `diff_threshold`, `diff_min_area` e `diff_max_changed`. Se o servidor é procurado na rede local diz-se em `server_mdns` (desligado por omissão). A leitura de uma peça com marca é `orientation_body_contrast`, `orientation_mark_contrast`, `orientation_min_mark_px` e `orientation_tolerance_deg`; a tolerância de um pad é `footprint_tolerance_mm`; o mapa de nitidez é `sharpmap_cells`, `sharpmap_min_share` e `sharpmap_min_contrast`; e a escala a partir de um tabuleiro é `board_square_mm` (a distância entre centros é `pitch_mm`), `board_max_aspect` e `board_max_spread`. Um `vision.cfg` que tenha um token deve manter-se privado. A nitidez que conta como 50 pontos num resultado de foco é `focus_reference`, e o passo ao longo da linha de uma aresta e a maior incerteza da sua posição são `edge_step_px` e `edge_max_uncertainty_px`. A câmara em direto, o tabuleiro de calibração e o alinhamento de uma placa têm as suas próprias chaves (`capture_*`, `calib_*`, `camera_offset_*` e `align_*`): cada uma tem um intervalo, um valor fora dele é recusado e um erro que uma chave governa nomeia a chave.

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

O conjunto tem 687 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

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
│   ├── batch.rs                                 uma pasta de imagens de uma vez, com um relatório CSV
│   ├── board.rs                                 a escala a partir de um tabuleiro de quadrados: ao longo de x, de y e a partir do lado, com relatório
│   ├── boardaim.rs                              uma placa posta onde o desenho diz: ver, mover, ver outra vez, sempre rumo ao mais seguro
│   ├── capture/                                 a câmara em direto: trait, formatos, propriedades, orçamento USB, saúde do fluxo, simulada
│   ├── charuco/                                 o tabuleiro de calibração com marcadores numerados: cantos, calibração, ficheiro e cartão
│   ├── config.rs                                o ficheiro de configuração: cada valor afinado
│   ├── correlate.rs                             marcas redondas encontradas por correlação com um disco do diâmetro pretendido
│   ├── diff.rs                                  duas imagens do mesmo sítio comparadas: o que mudou, onde e quanto
│   ├── cross.rs                                 marcas fiduciais em forma de cruz
│   ├── edge.rs                                  uma aresta com fração de píxel ao longo de uma linha, com a sua incerteza
│   ├── fiducial.rs                              as marcas redondas de uma imagem: limiar, manchas e verificações
│   ├── flatten.rs                               a luz irregular retirada de uma imagem
│   ├── footprint.rs                             cada pad julgado contra o tamanho do desenho, com uma tolerância em milímetros
│   ├── grid.rs                                  a distorção da objetiva a partir de uma grelha de marcas
│   ├── inspect.rs                               cada peça comparada com o corpo do seu encapsulamento a partir de uma tabela, com veredicto
│   ├── job.rs                                   o relatório de uma pasta lido num fio, com progresso e cancelamento
│   ├── lens.rs                                  os dois fatores da lente a partir de uma grelha de marcas, recusados se não encaixarem
│   ├── points.rs                                os ficheiros de pontos de um desenho
│   ├── quality.rs                               se uma imagem chega para medir: a sua nitidez, contraste e nível médio
│   ├── reference.rs                             uma imagem contra uma referência com uma máscara de tolerância
│   ├── histogram.rs                             o histograma e os cortes de Otsu, isodata e do método do triângulo
│   ├── report.rs                                o relatório de uma pasta como uma página HTML autónoma, em oito línguas
│   ├── ring.rs                                  as marcas em anel: redondas com um furo, de um ou dois tamanhos, com confiança
│   ├── limit.rs                                 leitura de um ficheiro com limite de tamanho e um código para cada recusa
│   ├── measure.rs                               peças medidas em milímetros, ficheiros CSV e JSON
│   ├── offset.rs                                onde está presa uma peça apanhada, com a correção, a tabela de um tabuleiro e o eixo do bico
│   ├── orientation.rs                           em que sentido está uma peça com marca, pela direção da marca
│   ├── pads.rs                                  uma fila de pads: o passo, quanto varia e quanto um pad se afasta da fila
│   ├── panels.rs                                os painéis da janela: objetiva, encapsulamentos, plano de imagens, desvios e verificação da imagem
│   ├── perspective.rs                           perspetiva corrigida: a imagem desenhada de novo vista de cima, a partir de quatro cantos
│   ├── scale.rs                                 de píxeis para milímetros e a distorção radial da objetiva
│   ├── server.rs                                o caminho até ao servidor EH3D pelo cliente partilhado e um servidor em memória para testes
│   ├── convention.rs                            o resultado de visão na convenção do servidor: fazê-lo, verificá-lo, SHA-256
│   ├── station.rs                               a estação de visão simulada do servidor: o trabalho, a resposta, uma portagem
│   ├── sharpmap.rs                              um mapa de onde a imagem é nítida: uma grelha de células, as fracas e a inclinação
│   ├── square.rs                                marcas quadradas e cruzes rodadas de qualquer ângulo
│   ├── tools.rs                                 as ferramentas do painel lateral sobre a imagem: formas, peças, relatório e passo
│   ├── shot.rs                                  o modo de captura da janela (--shot): os seus argumentos, lidos e verificados sem ecrã
│   ├── shots.rs                                 as posições da câmara que cobrem uma peça maior do que o campo, só com os tamanhos
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── checks.rs                                painéis da janela: correlação, endireitamento, fila de pads, comparação de imagens
│   ├── readings.rs                              painéis da janela: histograma, aresta, referência e definições do servidor
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: imagem, marcas, escala, objetiva e alinhamento
├── tests/
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   ├── read_limit.rs                            testes do limite de tamanho dos ficheiros que a linha de comandos lê
│   ├── exchange.rs                              testes dos comandos que leem uma imagem e escrevem um ficheiro
│   ├── commands.rs                              testes dos comandos correlate, rectify, pads, diff e server
│   ├── analysis.rs                              testes dos comandos orientation, footprint, sharpmap e board
│   ├── capture.rs                               câmara em direto, calibração e alinhamento na câmara simulada, e o comando
│   ├── fuzz.rs                                  testes com entrada aleatória e semente fixa: nada pode entrar em pânico
│   ├── camera.rs                                os novos comandos executados como programa: edge, histogram, refcheck, resíduos, server
│   ├── fuzz_camera.rs                           entrada aleatória com semente fixa para as leituras novas e a convenção
│   ├── fuzz_capture.rs                          entrada aleatória com semente fixa para leitores de imagem, calibração, alinhamento
│   ├── shot_sample.rs                           testes de que os exemplos abrem na janela de uma captura
│   ├── vision.rs                                testes da biblioteca e da linha de comandos
│   └── reqs.rs                                  testes do registo de requisitos
├── examples/
│   ├── design.csv                               um exemplo para experimentar o programa
│   ├── design_pair.csv                          um desenho para as duas marcas de marks.pgm
│   ├── marks.pgm                                um exemplo para experimentar o programa
│   ├── measured.csv                             um exemplo para experimentar o programa
│   ├── packages.csv                             uma tabela de corpos de encapsulamentos para o inspect
│   └── vision.cfg                               o ficheiro de configuração com cada valor e o seu significado
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   ├── NOT_YET.md                               boas ideias fora desta versão, com o porquê e o quando
│   ├── REQUIREMENTS.md                          o registo de requisitos: estado, evidência e teste de cada um
│   ├── SCOPE.md                                 para que serve, o que não fará e o que nunca decide
│   ├── STATUS.md                                o que existe, o que falta e o que continua em aberto
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── rust-toolchain.toml                          a toolchain de Rust fixada
├── CHANGELOG.md                                 o que mudou em cada versão
├── AGENTS.md                                    as regras para quem altera o repositório
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* A janela ainda não foi vista num ecrã; as marcas são emparelhadas com o desenho pela posição, por isso uma placa muito rodada ou uma marca em falta é recusada em vez de adivinhada.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente.
* Lê ficheiros de imagem, e uma câmara em direto só na simulação (os motores do Windows, do V4L2 e da libcamera são esqueletos nunca experimentados numa câmara real), só encontra marcas redondas, quadradas e em forma de cruz e foi experimentado com imagens desenhadas pelos testes, não com fotografias de placas reais. A rotação de uma peça é medida a partir dos seus píxeis como num retângulo, por isso uma peça que não é um retângulo recebe a rotação do seu eixo longo.
* O servidor não tem plugin de visão, por isso o programa usa os artefactos do armazém (`POST /api/v1/artifacts` para um resultado, `GET /api/v1/artifacts` e `GET /api/v1/artifacts/{id}` para a configuração de uma máquina) com nomes e um formato próprios (`vision-<kind>.json`, `vision-<machine>.cfg`); hoje só se testa contra um servidor em memória e um socket no loopback, e a procura do servidor por mDNS (`server_mdns`) é um interruptor sem resolvedor ligado.

**A seguir**

* Ler a câmara em direto da máquina (EH3D-NODE-BUNGARD-AUX), experimentar fotografias de placas reais e acrescentar à janela o treino de marcas novas.
* Ver a janela num ecrã e terminá-la.
* Experimentá-lo com ficheiros reais e com os outros programas da suíte.
* Escrever os motores em direto (imagens do Media Foundation, do V4L2 e da libcamera e o descodificador de JPEG em movimento), experimentar o tabuleiro de calibração com uma lente real e o alinhamento com uma mesa real.

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
* [Âmbito](docs/SCOPE.md): para que serve o programa, o que não fará e o que nunca decide.
* [Requisitos](docs/REQUIREMENTS.md): o registo das capacidades, o estado de cada uma e o teste que a mostra.
* [Estado](docs/STATUS.md): o que existe, o que falta e o que continua em aberto.
* [Ainda não](docs/NOT_YET.md): boas ideias que ficam fora desta versão.
* [Regras para alterar o repositório](AGENTS.md): as regras para quem altera o repositório.
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
