<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciales en una imagen de cámara: marcas, escala y corrección de una placa

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que funciona hoy:** una biblioteca Rust y un programa de línea de comandos (`eh3d-vision`) con 138 pruebas que se ejecutan en un ordenador: encuentra marcas fiduciales redondas en una imagen, convierte píxeles en milímetros y calcula el giro y el desplazamiento que colocan una placa donde dice su diseño. Tiene una ventana (se compila con `--features gui`, el programa `eh3d-vision-gui`) que abre una imagen, muestra las marcas con sus coordenadas, fija la escala, ajusta la distorsión del objetivo con una rejilla, alinea con el diseño y exporta las opciones de ISOCAM, pero todavía no se ha visto en una pantalla; no se ha probado en una máquina y no se ha comprobado con una colección de ficheros reales.

---

> **Estado de seguridad:** este programa no controla ninguna máquina y no es un sistema de seguridad. Lo que produce debe revisarlo una persona antes de ejecutar nada en una máquina; nada de esto se ha ejecutado en hardware real.


## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.3` |
| Madurez | Andamiaje |
| Pruebas | 138 |
| Pila tecnológica | Rust |
| En hardware real | No probado: nada de esto se ha ejecutado en una máquina |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura](#-estructura-del-repositorio) · [Relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Núcleo | 🟡 | encuentra marcas fiduciales redondas en una imagen, convierte píxeles en milímetros y calcula el giro y el desplazamiento que colocan una placa donde dice su diseño; probado en un ordenador con entradas normales y dañadas, no con una colección de ficheros reales. |
| Línea de comandos | 🟡 | `eh3d-vision` ejecuta las mismas funciones que la biblioteca e imprime el resultado; cada error vuelve como mensaje y un código de salida distinto de cero. |
| Pruebas | 🟡 | 138 pruebas pasan (`cargo test`), con casos normales, casos límite y entradas dañadas. |
| Seguridad | 🟡 | Rust seguro y simple (`forbid(unsafe_code)`): nada entra en pánico con los ficheros recibidos y un valor malo es un error con su motivo; sin revisión independiente. |
| Ventana | 🟠 | Escrita (egui, `--features gui`, el programa `eh3d-vision-gui`): abrir o soltar una imagen, ver las marcas con sus coordenadas, fijar la escala con una marca de tamaño conocido, ajustar la distorsión del objetivo con una rejilla, alinear con un diseño CSV y exportar las opciones de ISOCAM; su lógica está probada, pero todavía no se ha compilado ni visto en una pantalla. |
| Hardware real | 🔴 | No probado: nada de esto se ha ejecutado en una máquina. |
| Documentación | 🟡 | README en ocho idiomas, el registro de cambios y las dos imágenes dibujadas desde `docs/`. |

## 🎯 Resumen

* **Las marcas** (`src/fiducial.rs`): la imagen se parte en oscuro y claro con el umbral de Otsu, se miden las manchas conectadas y las que son discos del tamaño esperado (el área, lo que llenan del círculo que las contiene y lo alargadas que son) son las marcas, las mejores primero; una mancha cortada por el borde de la imagen se deja fuera porque no se puede fiar de su centro, y las marcas pueden ser oscuras sobre claro o claras sobre oscuro.
* **La escala y la lente** (`src/scale.rs`): los milímetros de un píxel salen de una marca de tamaño conocido, los ejes de la máquina suben mientras la imagen baja, y la distorsión radial de la lente se recupera por mínimos cuadrados desde una rejilla de marcas y se corrige.
* **La placa** (`src/align.rs`): con dos o más fiduciales del diseño y los mismos tal como los halló la cámara, se calculan por mínimos cuadrados el giro, el desplazamiento y (si se permite) el tamaño, con el residuo en milímetros; una transformación con residuo demasiado grande (una marca equivocada) o con el tamaño desviado (una placa estirada) no se acepta.

## 🧩 Papel en EH3D

EH3D-VISION-STUDIO es un programa de la EH3D Fabrication Suite. Funciona como biblioteca y línea de comandos y, compilado con `--features gui`, como ventana con el aspecto que comparten los demás programas de la suite. Lee y escribe ficheros simples, así que puede usarse solo o entre los demás programas.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-VISION-STUDIO: quién lo usa, qué contiene y de qué depende" width="100%">
</p>


Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba con el código.

## ✨ Funciones

* **Biblioteca y línea de comandos:** cada orden de `eh3d-vision` es una función de la biblioteca, así que otro programa de la suite puede llamarla sin lanzar un proceso.
* **Línea de comandos estricta:** una opción mal escrita (`--powr`) o una palabra de más es un error que la nombra, no algo que se ignora; si no, el programa haría su salida con los valores por defecto.
* **Pruebas con entradas dañadas:** las pruebas también dan ficheros vacíos, basura, números enormes y ficheros cortados, y ninguna puede provocar un pánico.
* **Sin adivinar:** un valor que no tiene sentido es un error que dice cuál y por qué, nunca un resultado que solo lo parece.
* **Sin código unsafe:** `#![forbid(unsafe_code)]` en la biblioteca y el programa.
* **Las marcas para ISOCAM:** `align --isocam-args` escribe cada par de marcas como la opción `--fiducial X_DISEÑO,Y_DISEÑO,X_MEDIDA,Y_MEDIDA` de `eh3d-isocam`.
* **Una ventana:** se compila con `--features gui` como el programa `eh3d-vision-gui` (en Windows se abre sin consola): abre una imagen PNG o PGM con zoom y desplazamiento, encuentra las marcas con valores que se pueden editar, las dibuja con sus coordenadas, fija la escala con una marca de tamaño conocido, ajusta la distorsión del objetivo con una rejilla de marcas, alinea con un diseño de un fichero CSV (las marcas se emparejan con él por su posición, así que el orden no importa) y exporta las opciones `--fiducial` de ISOCAM solo cuando la alineación se acepta. Habla los ocho idiomas y no necesita red.
* **Un fichero de configuración:** `vision.cfg` guarda cada valor que se afina (cómo es una marca, la rejilla del patrón, los límites de una alineación aceptada, el mayor fichero que se lee); un error en él se rechaza con su línea y `examples/vision.cfg` los enumera todos.

## 🔒 Seguridad y protección

| Amenaza | Defensa |
|---|---|
| Un fichero dañado, cortado o vacío | sin pánico: las pruebas cortan una entrada válida en cada posición y dan basura |
| Un número sin sentido (negativo, no finito, absurdamente grande) | se rechaza con un error que nombra el valor; nada se redondea hasta parecer plausible |
| Uso inseguro de memoria | Rust seguro y simple con `forbid(unsafe_code)` |

**Lo que no se defiende:** un resultado producido sin errores no es un resultado verificado. Nada de esto conoce los límites de una máquina y el programa no ha tenido revisión independiente. Informe de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La biblioteca es `eh3d_vision_studio` (`use eh3d_vision_studio::...`); la línea de comandos es `eh3d-vision`. Las órdenes son estas; cada una imprime su resultado por la salida estándar e informa de un error por la salida de errores con un código de salida distinto de cero. No hay acceso a la red.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuración

Los valores que se afinan viven en un fichero simple de `clave = valor`, no en el código: `vision.cfg` en la carpeta donde arranca el programa o junto a él (la variable `EH3D_VISION_CONFIG` nombra otro fichero). Fija el idioma de la ventana, si las marcas son oscuras o claras, el umbral, el rango del diámetro, el relleno y la elongación de una marca, el tamaño de la marca que fija la escala, las columnas y filas de la rejilla, si la alineación puede cambiar el tamaño, los límites que debe cumplir una alineación y el mayor fichero que se lee. `examples/vision.cfg` enumera cada clave con su significado y el valor con el que arranca el programa; si falta el fichero se usan esos valores y un fichero con un error se rechaza con su línea. En la línea de comandos lo mismo se da como opciones (véase Interfaces).

## 🚀 Inicio rápido

Necesita una cadena de herramientas Rust estable (1.88 o más reciente). El repositorio EH3D-COMMON debe estar clonado junto a este: el lector de imágenes se toma de él por ruta.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Para sacar una captura de la ventana sin abrirla a mano, ejecútelo con `--shot FICHERO.png` (también `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): se abre con ese tamaño, guarda el PNG, no guarda ajustes y se cierra solo (código de salida 0 si se escribió el fichero, 1 si no, 2 si los argumentos están mal).

## 🧪 Pruebas

El conjunto tiene 138 pruebas (`cargo test`, ejecutadas en Windows y Linux en el CI). Comprueban los casos normales, los casos límite y las entradas dañadas: ficheros vacíos, basura y ficheros cortados nunca provocan un pánico. El CI ejecuta además `cargo fmt --check`, `cargo clippy` con las advertencias como errores y `python -I tools/ci_validate.py`.

## 🏗️ Compilación y publicación

La versión está en `Cargo.toml` y en `eh3d.project.json`; cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilan el programa, `test.bat` / `test.sh` ejecutan las pruebas y `run.bat` / `run.sh` lo ejecutan con un ejemplo. Las versiones se publican en GitHub.

### Descargas

Cada versión trae el programa con su ventana para Windows (64 y 32 bits), Linux (64 bits) y macOS (Apple silicon e Intel), como `<nombre>-<versión>-<sistema>.zip` (Windows) o `.tar.gz` con el programa, la licencia, el léame, el icono y los ejemplos. Los ficheros los construye GitHub Actions, van en `SHA256SUMS` y están firmados con la clave de EH3D. Para comprobar una descarga, compara su hash con el fichero `.sha256` que la acompaña (`sha256sum -c`) y comprueba su firma con el fichero `.sig` que la acompaña (`ssh-keygen -Y verify`).

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   la biblioteca: lo que usa cada orden
│   ├── main.rs                                  la línea de comandos
│   ├── gui_main.rs                              la ventana como programa propio (característica gui)
│   ├── cli_args.rs                              los argumentos de la línea de comandos, el mismo fichero en cada utilidad pequeña
│   ├── align.rs                                 el giro, desplazamiento y tamaño de una placa y su emparejado con el diseño
│   ├── config.rs                                el fichero de configuración: cada valor afinado
│   ├── fiducial.rs                              las marcas redondas de una imagen: umbral, manchas y comprobaciones
│   ├── grid.rs                                  la distorsión del objetivo a partir de una rejilla de marcas
│   ├── points.rs                                los ficheros de puntos de un diseño
│   ├── scale.rs                                 de píxeles a milímetros y la distorsión radial del objetivo
│   ├── shot.rs                                  el modo de captura de la ventana (--shot): sus argumentos, leídos y comprobados sin pantalla
│   ├── browse.rs                                la lista de ficheros de la ventana que abre un fichero
│   ├── gui.rs                                   la ventana (egui, con la característica gui): dibuja el estado y transmite lo que hace la persona
│   ├── lang.rs                                  las palabras de la ventana en los ocho idiomas
│   ├── viewport.rs                              zoom y desplazamiento de la imagen, en números simples
│   └── window.rs                                el estado de la ventana sin pantalla: imagen, marcas, escala, objetivo y alineación
├── tests/
│   ├── readme.rs                                pruebas que ejecutan lo que prometen el README y los scripts run
│   ├── strict.rs                                pruebas de la línea de comandos: opciones y palabras que no conoce
│   ├── shot_sample.rs                           pruebas de que las muestras se abren en la ventana de una captura
│   └── vision.rs                                pruebas de la biblioteca y la línea de comandos
├── examples/
│   ├── design.csv                               una muestra para probar el programa
│   ├── marks.pgm                                una muestra para probar el programa
│   ├── measured.csv                             una muestra para probar el programa
│   └── vision.cfg                               el fichero de configuración con cada valor y su significado
├── docs/
│   ├── architecture.json                        datos de la imagen de Arquitectura
│   └── structure.json                           datos de la imagen de Estructura
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── rust-toolchain.toml                          la toolchain de Rust fijada
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* La ventana todavía no se ha visto en una pantalla; las marcas se emparejan con el diseño por su posición, así que una placa muy girada o una marca que falta se rechaza en lugar de adivinarse.
* No se ha probado en una máquina ni con una colección de ficheros reales y no ha tenido revisión independiente.
* Lee un fichero de imagen, no una cámara en vivo, solo encuentra marcas redondas (ni cruces ni cuadrados) y se ha probado con imágenes dibujadas por las pruebas, no con fotografías de placas reales.

**Siguiente**

* Leer la cámara en vivo de la máquina (EH3D-NODE-BUNGARD-AUX), encontrar cruces y cuadrados, probar fotografías de placas reales y añadir a la ventana el entrenamiento de marcas nuevas.
* Ver la ventana en una pantalla y terminarla.
* Probarlo con ficheros reales y con los demás programas de la suite.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): los crates compartidos, entre ellos el lector de imágenes
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): el montaje pick and place
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): el fresado de aislamiento del cobre
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): el programa de control de máquina
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): el nodo del cabezal de taladrado con la cámara
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): el centro que encuentra y arranca los programas de la suite

## 📚 Documentación

* [Registro de cambios](CHANGELOG.md)
* [Datos de arquitectura y estructura](docs/architecture.json): los datos con los que se dibujan las dos imágenes.

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `python -I tools/ci_validate.py`: todo debe pasar. Véanse [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio de texto se hace en los ocho ficheros, y las dos imágenes se dibujan desde `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Informe de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
