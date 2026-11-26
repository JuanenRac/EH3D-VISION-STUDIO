<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciales en una imagen de cámara: marcas, escala y corrección de una placa

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que funciona hoy:** una biblioteca Rust y un programa de línea de comandos (`eh3d-vision`) con 967 pruebas que se ejecutan en un ordenador: encuentra marcas fiduciales redondas en una imagen, convierte píxeles en milímetros y calcula el giro y el desplazamiento que colocan una placa donde dice su diseño. Tiene una ventana (se compila con `--features gui`, el programa `eh3d-vision-gui`) que abre una imagen, muestra las marcas con sus coordenadas, fija la escala, ajusta la distorsión del objetivo con una rejilla, alinea con el diseño y exporta las opciones de ISOCAM, pero todavía no se ha visto en una pantalla; no se ha probado en una máquina y no se ha comprobado con una colección de ficheros reales.

---

> **Estado de seguridad:** este programa no controla ninguna máquina y no es un sistema de seguridad. Lo que produce debe revisarlo una persona antes de ejecutar nada en una máquina; nada de esto se ha ejecutado en hardware real.


## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.1.4` |
| Madurez | Funcional |
| Pruebas | 967 |
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
| Pruebas | 🟡 | 967 pruebas pasan (`cargo test`), con casos normales, casos límite y entradas dañadas, y una cámara simulada sobre una placa que añade ruido, mala exposición, un fotograma muerto o ausente, polvo que parece una marca o un archivo de imagen cortado, y se vuelve a pedir. |
| Seguridad | 🟡 | Rust seguro y simple (`forbid(unsafe_code)`; un pequeño borde unsafe solo en los motores de cámara compilados con su característica): nada entra en pánico con los ficheros recibidos y un valor malo es un error con su motivo; sin revisión independiente. |
| Ventana | 🟠 | Escrita (egui, `--features gui`, el programa `eh3d-vision-gui`): abrir o soltar una imagen, ver las marcas con sus coordenadas, fijar la escala con una marca de tamaño conocido, ajustar la distorsión del objetivo con una rejilla, alinear con un diseño CSV y exportar las opciones de ISOCAM; su lógica está probada, pero todavía no se ha compilado ni visto en una pantalla. |
| Cámara en vivo | 🟠 | la capa de cámara, el tablero de calibración y la alineación de una placa están escritos y probados con una cámara y una mesa simuladas; los motores de Windows, V4L2 y libcamera son esqueletos y nada se ha probado con una cámara, una lente o una placa reales. |
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

## 🖼️ Capturas de pantalla

<p align="center">
  <img src="images/screens/vision-studio-01-window.png" alt="La ventana con la imagen y el diseño de ejemplo" width="100%">
</p>

<p align="center"><em>La ventana con la imagen y el diseño de ejemplo: las dos marcas encontradas, cada una con su centro y su número, y los ajustes de la búsqueda a la izquierda.</em></p>

## ✨ Funciones

* **Biblioteca y línea de comandos:** cada orden de `eh3d-vision` es una función de la biblioteca, así que otro programa de la suite puede llamarla sin lanzar un proceso.
* **Línea de comandos estricta:** una opción mal escrita (`--powr`) o una palabra de más es un error que la nombra, no algo que se ignora; si no, el programa haría su salida con los valores por defecto.
* **Pruebas con entradas dañadas:** las pruebas también dan ficheros vacíos, basura, números enormes y ficheros cortados, y ninguna puede provocar un pánico.
* **Sin adivinar:** un valor que no tiene sentido es un error que dice cuál y por qué, nunca un resultado que solo lo parece.
* **Límite de tamaño de los ficheros que lee:** la línea de comandos lee una imagen o un fichero de puntos solo si no pesa más que `max_file_mib` de `vision.cfg` (64 MiB por defecto); un fichero mayor se rechaza con el código `file_too_large` y su nombre, uno que no se puede leer con `file_unreadable` y un fichero de puntos que no es texto con `file_not_text`, y no se carga nada.
* **Sin código unsafe en la compilación por defecto:** `#![forbid(unsafe_code)]` en la biblioteca y el programa; solo los motores de cámaras en vivo, compilados con su propia característica, tienen un pequeño borde unsafe (unas pocas llamadas, con los tamaños de las estructuras comprobados al compilar).
* **Las marcas para ISOCAM:** `align --isocam-args` escribe cada par de marcas como la opción `--fiducial X_DISEÑO,Y_DISEÑO,X_MEDIDA,Y_MEDIDA` de `eh3d-isocam`.
* **Una ventana:** se compila con `--features gui` como el programa `eh3d-vision-gui` (en Windows se abre sin consola): abre una imagen PNG o PGM con zoom y desplazamiento, encuentra las marcas con valores que se pueden editar, las dibuja con sus coordenadas, fija la escala con una marca de tamaño conocido, ajusta la distorsión del objetivo con una rejilla de marcas, alinea con un diseño de un fichero CSV (las marcas se emparejan con él por su posición, así que el orden no importa) y exporta las opciones `--fiducial` de ISOCAM solo cuando la alineación se acepta. Habla los ocho idiomas y no necesita red.
* **Un fichero de configuración:** `vision.cfg` guarda cada valor que se afina (cómo es una marca, la rejilla del patrón, los límites de una alineación aceptada, el mayor fichero que se lee); un error en él se rechaza con su línea y `examples/vision.cfg` los enumera todos.
* **El centro con fracción de píxel y una confianza:** `find --refine` (o `refine_subpixel` en `vision.cfg`) pondera los píxeles grises del borde de cada marca, así que su centro se halla con una fracción de píxel, y escribe una confianza que baja cuando la marca apenas destaca de la placa.
* **Luz desigual:** `--flatten PX` (o `flatten_window`) divide la imagen por la luz que rodea a cada píxel antes de la búsqueda, de modo que una lámpara que se apaga por un lado de la placa o una sombra no escondan las marcas del lado oscuro.
* **Marcas en forma de cruz:** `eh3d-vision crosses IMAGEN` halla signos más con los brazos a lo largo de los ejes de la imagen (caja cuadrada, dos brazos del mismo ancho, esquinas vacías); los discos y los cuadrados se descartan y `--turned` halla cruces con cualquier giro (véase abajo).
* **La escala desde el paso de un patrón:** `eh3d-vision calibrate IMAGEN --pitch-mm D` toma la mediana de la distancia entre marcas vecinas de una rejilla, rechaza un patrón que no se ve de frente y escribe la línea `mm_per_px = ...` para `vision.cfg`.
* **Piezas para otros programas, en ficheros simples:** `eh3d-vision measure IMAGEN --mm-per-px V` da de cada pieza el centro (mm desde el centro de la imagen, x a la derecha, y arriba), el giro (de -90 sin incluir a 90 grados), el largo, el ancho y el área, en CSV (`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`) o en JSON (`--format json`: `format` `eh3d-vision-parts`, `version` 1, `unit`, `image_width_px`, `image_height_px`, `mm_per_px`, `parts`); no se comparte nada más con el programa que los lee. `--output FICHERO` nunca escribe sobre un fichero sin `--replace`, ni sobre un enlace.
* **Una carpeta de una vez:** `eh3d-vision batch CARPETA --report FICHERO.csv` lee cada imagen PNG y PGM dentro del límite de tamaño (como mucho `batch_max_files`) y escribe una línea por imagen (`file,status,marks,mean_score,min_score,mean_diameter_px,detail`) con el estado `ok`, `no_marks`, `picture_unreadable` o el código del rechazo; un nombre de fichero que una hoja de cálculo ejecutaría como fórmula se escribe con una comilla delante.
* **Cuadrados y cruces con cualquier giro:** `eh3d-vision squares IMAGEN` halla marcas cuadradas giradas cualquier ángulo (centro, lado y el ángulo antihorario como lo ve la máquina, de -45 sin incluir a 45 grados) y `eh3d-vision crosses IMAGEN --turned` halla cruces cuyos brazos no van por los ejes; el relleno mínimo de un cuadrado y la puntuación mínima de una cruz (`square_min_fill`, `cross_min_score`) están en `vision.cfg`.
* **Marcas en anillo de un tamaño o de dos:** `eh3d-vision rings IMAGEN` halla marcas redondas con un agujero (donuts): cada una trae su centro, su diámetro exterior e interior, la puntuación de su forma, una confianza (la puntuación rebajada cuando el anillo destaca poco del fondo, `full_contrast`) y, cuando los anillos son de dos tamaños, si es pequeño o grande (`--split PX` o `ring_split` da el diámetro que los separa; si no, se halla donde los diámetros caen en dos grupos con un hueco de al menos `ring_size_gap`). Se descartan un disco, un anillo con un hueco, un anillo cortado por el borde, uno que no es redondo y uno cuyo agujero es demasiado pequeño o grande (`ring_min_ratio`, `ring_max_ratio`). La ventana tiene una sección para ellos que dibuja el círculo exterior y el interior con un color para cada tamaño.
* **Dónde se sostiene una pieza recogida:** `eh3d-vision offset IMAGEN --package NOMBRE` toma una imagen de una pieza vista desde abajo (una sola pieza), la mide como `measure`, compara su tamaño con el cuerpo de su paquete como `inspect` y da su desplazamiento respecto al eje de la boquilla (`--nozzle-x`, `--nozzle-y` o `nozzle_x_mm`, `nozzle_y_mm`, milímetros desde el centro de la imagen, x a la derecha e y hacia arriba), su giro respecto a como debería sostenerse (`--nominal-angle`; una pieza tan larga como ancha se gira por el camino corto dentro de un cuarto de vuelta, cualquier otra dentro de media vuelta) y la corrección que la deja donde la boquilla la querría: las mismas distancias y giro al revés. CSV o JSON, sin escribir nunca sobre un fichero sin `--replace`; el código de salida es 1 cuando la pieza tiene mal el tamaño, se sostiene más lejos de `--max-offset` (`pick_max_offset_mm`) o gira más de `--max-turn` (`pick_max_turn_deg`), una vez escrito el fichero, y se rechaza una imagen sin pieza o con más de una (`no_parts`, `parts_count`). Es geometría sobre una imagen: no se usó cámara, boquilla ni máquina.
* **Una tabla de correcciones para una bandeja y un panel de la ventana para ella:** `eh3d-vision offset IMAGEN --package NOMBRE --each` toma cada pieza de la imagen (una bandeja), cada una por su cuenta, y escribe una tabla con la columna `part` (el número de la pieza, desde 1) y luego las columnas de `offset`, una fila por pieza; el código de salida es 1 cuando alguna pieza falla. La ventana tiene un panel para lo mismo (`nozzle_x_mm`, `nozzle_y_mm`, `nominal_angle_deg`, `pick_max_offset_mm`, `pick_max_turn_deg`): calcula la tabla de la imagen abierta, enumera la corrección de cada pieza en el idioma de la ventana y la guarda como `<imagen>-offsets.csv`. `nominal_angle_deg` es el giro de una pieza bien sostenida, también en `vision.cfg`.
* **Una cámara en espejo y el eje de la boquilla:** `--mirror` (`pick_mirror`) es para una cámara que ve las piezas dadas la vuelta de izquierda a derecha: la x y el giro de cada pieza cambian de signo. `eh3d-vision nozzle IMAGEN IMAGEN... --package NOMBRE` toma imágenes de piezas recogidas con la misma boquilla (una pieza en cada una, al menos dos imágenes) y escribe las líneas `nozzle_x_mm` y `nozzle_y_mm` para `vision.cfg`, el lugar medio de las piezas (no depende del eje que se usaba al medirlas); por la salida de errores dice cuánto se dispersan las piezas (la desviación típica en x y en y, la pieza más lejana y el giro medio).
* **Si la imagen basta para medir:** `eh3d-vision quality IMAGEN` da la nitidez (la varianza del laplaciano), el contraste (la desviación típica de los niveles de gris) y el nivel de gris medio de una imagen, en CSV o JSON, y la rechaza (código de salida 1, tras escribir el fichero) como `flat`, `dark`, `bright` o, cuando `--min-sharpness` es mayor que 0, `blurry`; los mínimos son `--min-contrast`, `--min-mean`, `--max-mean` y `--min-sharpness`, o las claves de `vision.cfg`. Con `quality_check = yes`, `offset`, `nozzle` y la ventana rechazan una imagen mala (`poor_picture`) antes de medir una pieza en ella, y la ventana tiene un panel para comprobar la imagen y mostrar sus números. Los valores dependen de la cámara y del objetivo, por eso están en el fichero de configuración y la nitidez empieza desactivada; nada de esto se ha probado con una cámara real.
* **La lente con dos factores, rechazada si no encaja:** `eh3d-vision lens IMAGEN` calcula `k1` y `k2` de la distorsión radial desde una rejilla de marcas y escribe las líneas `lens_k1 = ...` y `lens_k2 = ...` para `vision.cfg`; rechaza un ajuste que deja las marcas a más de `lens_max_rms` píxeles de su retícula (`lens_residual_too_large`) o un factor mayor que 0,5 (`lens_out_of_range`), y `find --undistort` endereza las posiciones con ellos.
* **Un veredicto para cada pieza:** `eh3d-vision inspect IMAGEN --package NOMBRE --table packages.csv` mide las piezas de una imagen y compara cada una con el cuerpo de su encapsulado (`package`, `body_long_mm`, `body_short_mm` y, si se quiere, `tolerance_mm`, en un fichero CSV; `examples/packages.csv` trae algunos): `pass` o `fail` con el motivo (`length`, `width`, `angle`) en CSV o JSON, y el código de salida 1 cuando una pieza falla; la tolerancia y el giro máximo (`inspect_tolerance`, `inspect_max_angle`) están en `vision.cfg`.
* **Un informe HTML de la carpeta:** `eh3d-vision batch CARPETA --html informe.html` escribe una página que se basta sola (sin scripts, sin enlaces, sin otros ficheros): una tabla de las imágenes, un dibujo pequeño de cada una con un círculo en cada marca hallada y los avisos (una imagen que no se pudo usar, sin marcas o con una puntuación por debajo de `report_low_score`), en el idioma de `--lang` o de `vision.cfg`.
* **Las herramientas nuevas en la ventana:** el panel lateral trae el centro con fracción de píxel con la confianza de cada marca, la ventana de la luz, cuadrados y cruces con cualquier giro (dibujados sobre la imagen), las piezas en milímetros (guardadas como `<imagen>-parts.csv`), el informe de la carpeta, leído en un hilo con barra de progreso y botón de cancelar (`eh3d-vision-report.html`, que nunca se escribe sobre un fichero sin `replace_output`) y la escala desde el paso de un patrón, con los textos en los ocho idiomas y los valores en `vision.cfg`; todavía no se ha visto en una pantalla.
* **El objetivo en la ventana, con los dos factores:** el botón de la rejilla ajusta `k1` y `k2` como hace `eh3d-vision lens` (se rechaza por encima de `lens_max_rms`, un campo del panel), la ventana dibuja los lugares donde el objetivo ajustado pone la retícula de la rejilla, para compararlos con las marcas encontradas (`show_lens_grid`), toma los factores de `lens_k1` y `lens_k2` de `vision.cfg` sin rejilla y los guarda como `<imagen>-lens.cfg`.
* **Piezas frente a su paquete e imágenes de una pieza grande, en la ventana:** el panel lateral lee la tabla de paquetes (`package_table`), juzga las piezas medidas frente al paquete `inspect_package` (línea verde o roja por pieza, con el motivo del fallo; `inspect_tolerance` e `inspect_max_angle` son campos del panel) y guarda los veredictos como `<imagen>-inspection.csv`, el fichero de `eh3d-vision inspect`; y planifica las imágenes que necesita una pieza mayor que el campo (el campo es la imagen en milímetros, la pieza es `shots_part_width` por `shots_part_height` o el cuerpo del paquete, con `shots_overlap` y `shots_margin`), lista adónde va la cámara y guarda `<imagen>-shots.csv`, el fichero de `eh3d-vision shots`. La línea de comandos toma las mismas dos claves nuevas cuando no se da `--package` ni el tamaño de la pieza.
* **Las imágenes de una pieza grande, planificadas:** `eh3d-vision shots --package NOMBRE --fov-w MM --fov-h MM` (o `--body-long` y `--body-short`) lista en CSV los lugares de la cámara, en milímetros desde el centro de la pieza, que ven toda una pieza mayor que el campo, con las imágenes vecinas solapadas en `shots_overlap` y un margen de `shots_margin`; no necesita cámara.
* **Marcas por correlación:** `eh3d-vision correlate IMAGEN --diameter PX` desliza un disco de ese diámetro sobre la imagen y calcula la correlación normalizada en cada lugar (no cambia si la luz es más fuerte o el contraste menor), así que se encuentra una marca que el umbral pierde con luz desigual; da los lugares con fracción de píxel y la puntuación, la mejor primero, ninguno más cerca de uno mejor que un diámetro. Necesita el diámetro (`correlate_diameter`, `correlate_min_score`) y es más lenta que la búsqueda por umbral, así que se rechaza una imagen que tardaría demasiado (`correlate_too_big`).
* **Perspectiva corregida:** `eh3d-vision rectify IMAGEN --corners X1,Y1,...,X4,Y4 --output FICHERO.pgm` dibuja de nuevo la imagen como se vería desde justo arriba, a partir de las cuatro esquinas de un rectángulo en ella (una homografía con muestreo bilineal); unas esquinas que no forman un cuadrilátero convexo se rechazan (`bad_corners`) y el lado mayor es `rectify_max_side`.
* **Una fila de pads:** `eh3d-vision pads IMAGEN --mm-per-px V` toma las piezas de la imagen como pads y da el paso (la distancia mediana a lo largo de la recta de mejor ajuste), cuánto varía, cuánto se aparta un pad de la fila y su tamaño medio, en CSV o JSON; el veredicto es `fail` con los motivos `pitch`, `row` y `expected` (`--expect-pitch`), según `pads_tolerance`, y el código de salida es 1.
* **Antes y después:** `eh3d-vision diff ANTES DESPUÉS` compara dos imágenes del mismo tamaño: los píxeles cuyo nivel de gris difiere en más de `diff_threshold`, su proporción, la diferencia media y la mayor, el recuadro que los rodea y las regiones de al menos `diff_min_area` píxeles; `--mask FICHERO.pgm` escribe la imagen de los cambios y el código de salida es 1 cuando cambió más de `diff_max_changed`.
* **En qué sentido está sujeta una pieza:** `eh3d-vision orientation IMAGEN --mm-per-px V` busca la pieza (lo que difiere de la placa) y la marca que lleva (la banda de un diodo, el punto del pin 1: lo que difiere del cuerpo dentro de ella, aunque se parezca a la placa) y da la dirección desde el centro de la pieza hasta la marca (grados en sentido antihorario desde el eje x, y hacia arriba), el cuarto de vuelta más cercano y cuánto se aparta de él y, con `--expect-deg`, cuánto se aparta de la dirección deseada. La pieza se rechaza (código de salida 1, después de escribir el fichero) como `no_mark`, `centred`, `mark_big`, `wrong_way` o, sin dirección deseada, `skewed`.
* **La huella de los pads:** `eh3d-vision footprint IMAGEN --pad-length MM --pad-width MM` mide cada pad y juzga cada uno por separado frente al diseño con una tolerancia en milímetros (`--tolerance`, `footprint_tolerance_mm`): una línea por pad, de la fila de arriba hacia abajo y de izquierda a derecha, con las diferencias y los motivos `length` y `width`; `--count` comprueba también el número de pads (`count`) y el código de salida es 1 cuando un pad se sale.
* **Un mapa de dónde es nítida la imagen:** `eh3d-vision sharpmap IMAGEN --cells N` corta la imagen en una rejilla y da la nitidez de cada celda (la varianza del laplaciano), la mediana, la celda más débil, la inclinación de la nitidez a lo largo de x y de y y las celdas débiles (por debajo de `--min-share` de la mediana); una celda casi sin contraste está `empty` y no se juzga. `--heat FICHERO.pgm` escribe la imagen del mapa, oscura donde la imagen está borrosa, y el código de salida es 1 cuando hay una celda débil (`uneven`).
* **La escala desde un tablero de cuadrados, con informe:** `eh3d-vision board IMAGEN --pitch-mm D --square-mm S` busca los cuadrados, calcula el giro del tablero y la distancia entre vecinos a lo largo de cada eje, y da los milímetros de un píxel a lo largo de x, de y y desde el lado, con lo cuadrados que son los píxeles (`aspect`) y cuánto difieren las distancias (`spread`); `--report FICHERO.txt` escribe el informe para una persona con la línea `mm_per_px = ...` para `vision.cfg`. Se rechaza (código de salida 1) cuando los píxeles no son cuadrados (`aspect`), las distancias difieren (`spread`) o el lado y el paso no concuerdan (`disagree`).
* **Cuatro paneles más en la ventana:** el panel lateral tiene la búsqueda por correlación (las marcas halladas se listan con su lugar y su puntuación), la imagen enderezada con las cuatro marcas halladas (se guarda como `<imagen>-rectified.pgm`), la fila de pads juzgada con las piezas medidas (se guarda como `<imagen>-pads.csv`) y la comparación de la imagen abierta con otra (se guarda como `<imagen>-diff.csv` y `<imagen>-changes.pgm`); sus valores son campos de los paneles (`correlate_diameter`, `correlate_min_score`, `rectify_max_side`, `pads_tolerance`, `diff_threshold`, `diff_min_area`, `diff_max_changed`) y sus textos están en los ocho idiomas. Su lógica está probada; la ventana no se ha visto en una pantalla.
* **El servidor de la máquina:** `eh3d-vision server health|config|discover|check|detect|send` usa las mismas claves que los demás programas de la suite (`server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns`) a través del cliente compartido de la suite (`eh3d-serverclient` de EH3D-COMMON): `config` lee los valores de `vision.cfg` que el servidor guarda para la máquina (el artefacto más reciente `vision-<machine>.cfg` del formato `text/x-eh3d-vision-config`, escrito como las líneas del fichero o como un objeto JSON; todo o nada; las claves del enlace nunca se toman del servidor) y los escribe como líneas para el fichero, y `send quality|offset|pads|diff|orientation|footprint|sharpmap|board` guarda en el servidor el resultado JSON de esa orden como el artefacto `vision-<kind>.json` (`POST /api/v1/artifacts`, con la máquina y el tipo en sus parámetros). Los reintentos y el estado del enlace son los del cliente compartido (una petición que pudo llegar nunca se envía dos veces), los rechazos del servidor se pueden decir por su código en ocho idiomas, el token nunca se imprime y se avisa de un token que iría por HTTP plano a una dirección que no es local. El servidor tiene un complemento `vision`, así que los resultados también se guardan en su convención (las viñetas siguientes); se prueba contra un servidor en memoria (`server::SimServer`) y un socket en el bucle local, no contra el servidor real.
* **Resultados en la convención del servidor:** `eh3d-vision server send fiducials|focus|blobs IMAGEN` guarda un resultado en la forma que comparten el complemento `vision` de EH3D-SERVER y los programas que miran por una cámara: un fichero `vision-<tipo>.json` de formato `application/json` y esquema `eh3d.vision.result/1`, con el tamaño y el SHA-256 de la imagen, los hallazgos del tipo (`fiducials` `{x, y, radius_px, score}`, `focus` `{score, sharp}`, `blobs` `{x, y, area_px}`; `barcode` se comprueba pero nunca se hace, porque aquí nada lee códigos), las unidades (`px`) y la evidencia (`picture`, salvo que `--evidence camera|simulator` diga otra cosa: el programa no puede saber que un fichero vino de una cámara). `server check FICHERO.json` dice si cualquier documento sigue la convención y nombra la parte que no, y un documento que no la sigue se rechaza antes de enviarlo. La puntuación de una comprobación de enfoque es 50 cuando la nitidez es `focus_reference`.
* **La estación simulada del servidor:** `eh3d-vision server detect fiducials|barcode|focus|blobs IMAGEN [--seed N] [--artifact]` pide al complemento `vision` del servidor (`POST /api/v1/work/jobs`, iniciado y leído hasta que termina) el resultado que inventa a partir del tamaño y el hash de la imagen y la semilla, siempre el mismo (`evidence: simulator`), en la vista del trabajo o guardado como artefacto. `station::simulate` es un port del complemento con el que responde el servidor en memoria, de modo que toda la cadena de trabajo, resultado y artefacto se prueba aquí sin red; no se ha ejecutado contra el servidor real.
* **La búsqueda del servidor en la red, como interruptor real:** `eh3d-vision server discover` lista el `host:puerto` de los servidores que responden a `_eh3d._tcp` en la red local. No hace nada mientras `server_mdns` está apagado (por defecto), y el programa debe compilarse con `--features mdns` (los programas de las versiones lo están) para tener el resolutor del cliente compartido. Lo que encuentra son candidatos para que la persona elija, nunca se conecta solo a ellos, porque cualquier dispositivo de la red puede responder.
* **Marcas entrenadas con un ejemplo, una placa muy girada y una lista segura de servidores:** `eh3d-vision train IMAGEN --rect X,Y,W,H --output FICHERO` guarda el promedio de rectángulos recortados alrededor de una marca como el fichero de una marca (nunca se escribe sobre un fichero que existe salvo con `--replace`), y `eh3d-vision locate --mark FICHERO IMAGEN` la vuelve a encontrar por correlación, girada paso a paso si se pide, con su posición, su ángulo y su puntuación (las claves `train_*` de `vision.cfg`); `eh3d-vision pair DISEÑO.csv MEDIDAS.csv --any-turn` empareja las marcas con el diseño sea cual sea el giro de la placa, con una marca que falta o una de más, y rechaza lo que dos formas explican igual de bien (las claves `pair_*`); `discovery` convierte lo que responde en la red en pocos candidatos locales bien formados (las claves `find_server_*`), y `server discover` y la ventana listan solo esos. Las órdenes responden en el idioma de `vision.cfg` o de `--lang`, dan JSON con `--json` y terminan con el código de salida 0 (hecho), 1 (no se pudo llevar a cabo) o 2 (se llevó a cabo, pero la comprobación no se superó: ejemplos que no concuerdan, una marca que no se distingue de sus vecinas, una marca que no se encuentra, marcas que no se pueden emparejar). Todo se prueba con imágenes dibujadas y la verdad que las dibujó; el bucle de mirar, mover y volver a mirar sigue emparejando por posición y la ventana todavía no entrena marcas.
* **Un lector de fotogramas JPEG en movimiento, una tarjeta de puntos para imprimir, la luz de una imagen y los agujeros de una cinta:** `eh3d-vision jpeg FOTOGRAMA` lee un fotograma JPEG base hasta su brillo dentro de `mjpeg_max_bytes` y `mjpeg_max_pixels`, y un fotograma cortado, con un marcador donde no puede haberlo, una tabla imposible o un tamaño de miles de millones de píxeles se rechaza con un código que no cambia (`jpeg_truncated`, `jpeg_bad_marker`, `jpeg_bad_table`, `jpeg_bad_size`, `jpeg_unsupported`, `jpeg_bad_data`), nunca con un pánico; `eh3d-vision calib-card` dibuja una tarjeta de puntos redondos en milímetros para imprimir al 100 %, igual cada vez y rechazada si no cabe en su página (las claves `card_*`); `eh3d-vision light-check IMAGEN` dice si la luz de una imagen es buena (media, parte quemada, zonas, inclinación; las claves `light_*`) y por qué no, con el código de salida 2 si no lo es; `eh3d-vision tape-locate IMAGEN` encuentra los agujeros de una cinta de piezas, el paso frente al paso de la cinta y los bolsillos (las claves `tape_*`). Todo se prueba con imágenes y fotogramas dibujados con una semilla fija: no se usó ninguna cámara, no se imprimió ninguna tarjeta ni se fotografió ninguna cinta.
* **Un borde con fracción de píxel y su incertidumbre:** `eh3d-vision edge IMAGEN --from X,Y --to X,Y` lee la imagen a lo largo de una línea trazada a través de un borde (bilineal, separadas `edge_step_px`), toma los dos extremos de la línea como los dos fondos planos, halla dónde el perfil cruza el nivel intermedio y da la posición a lo largo de la línea y en la imagen, el contraste, el ancho de la transición y la incertidumbre (una desviación típica: el ruido de los fondos, con el redondeo de 8 bits añadido, dividido por la pendiente donde cruza). El código de salida es 1 cuando la incertidumbre supera `edge_max_uncertainty_px`; una línea sin borde es `edge_flat`. Un borde más nítido que un píxel puede desviarse hasta 0,05 px.
* **El histograma y el umbral automático:** `eh3d-vision histogram IMAGEN --bins N` da los niveles de gris en 8 a 256 grupos y los cortes de Otsu (el que usa la búsqueda de marcas), isodata y el método del triángulo, cuánta varianza explican los dos grupos de Otsu, la proporción de píxeles negros y blancos y si los tres cortes coinciden (dentro de 8 niveles); si no coinciden, conviene revisar la iluminación o el `threshold` de `vision.cfg`.
* **Una imagen contra una referencia, con una máscara de tolerancia:** `eh3d-vision refcheck REFERENCIA IMAGEN --tolerance MASCARA.pgm` compara dos imágenes del mismo tamaño donde la diferencia permitida puede cambiar de un sitio a otro: un píxel de la máscara en 0 mantiene el `diff_threshold` general, de 1 a 254 permite esos niveles de gris más, y 255 no se mira. Da los píxeles fuera de la tolerancia, cuánto la superan, en cuántas regiones (`diff_min_area`) y dónde; `--map FICHERO.pgm` los escribe como imagen y el código de salida es 1 por encima de `diff_max_changed`.
* **Los residuos de la lente, marca a marca:** `eh3d-vision lens IMAGEN --residuals` escribe, para cada marca de la cuadrícula, cuánto se aleja de su sitio antes y después de la corrección, la marca más lejana, las tres bandas de distancia al centro y la razón de la banda exterior a la interior: una razón muy por encima de 1 indica que dos términos radiales no bastan para la lente (o que el patrón no es plano).
* **Más en la ventana:** el panel lateral tiene los ajustes del servidor (el panel de la suite de `eh3d-view`: dirección, token, máquina, reintentos, el interruptor de la búsqueda en la red, inicio de sesión y sesiones), el estado del enlace en la esquina, la búsqueda de servidores (solo con el interruptor activado, en un hilo, con candidatos para elegir), un histograma con los tres cortes, el borde a lo largo de una línea de cuatro números, la comprobación contra una referencia con máscara de tolerancia, y una línea de ayuda plegada bajo los títulos de los paneles, en los ocho idiomas. La ventana no se ha visto en una pantalla.
* **Pruebas con semilla fija:** `tests/fuzz.rs` da imágenes de cualquier tamaño, esquinas en cualquier sitio, pads de cualquier tamaño, cuadrados en cualquier lugar y ficheros de configuración, JSON, Base64 y respuestas HTTP dañados, desde un generador que no lee el reloj; nada debe entrar en pánico y lo que sale debe tener sentido, y los paneles de la ventana pasan igual por imágenes aleatorias. `tests/fuzz_camera.rs` hace lo mismo con un borde a lo largo de cualquier línea de cualquier imagen, el histograma, una imagen contra una referencia con cualquier máscara, los residuos de cualquier conjunto de marcas y documentos de la convención cortados, alterados o llenos de basura.
* **Una cámara en vivo detrás de un trait, probada solo con una simulada (`src/capture/`):** las cámaras se listan por nombre con una clave que no cambia (`vendor:product:serial@port`), se abren en un formato (YUYV, JPEG en movimiento, RGB, gris) y dan imágenes con un contador y sus bytes originales; cada una de las 13 propiedades tiene su rango o la cámara dice que no la tiene. Se encuentran la carga de un bus USB (`capture_usb_mbps`), las imágenes negras, repetidas, perdidas o cortadas y el parpadeo de la luz, se espera a imágenes nuevas por su cuenta y no por segundos, y el registro va a una función. La cámara simulada falla a propósito; los motores de Windows, V4L2 y libcamera son esqueletos tras las características `mfoundation`, `v4l2` y `libcamera` (apagadas por defecto) y no se han probado con una cámara real, y las imágenes de JPEG en movimiento se comprueban pero aún no se decodifican.
* **Un tablero de calibración con marcadores numerados (`src/charuco/`):** `eh3d-vision charuco card` dibuja un tablero de ajedrez cuyas casillas blancas llevan marcadores, en milímetros para imprimir al 100 %; `eh3d-vision charuco calibrate IMAGE` lee los marcadores, encuentra las esquinas con una fracción de píxel con cualquier giro y ajusta el mapa de píxeles a milímetros de tres maneras (homografía, afín, similitud). Se rechaza si deja demasiado en las esquinas (`calib_max_residual_px`), si los métodos separan demasiado las esquinas (`calib_max_method_gap`) o si no coincide con la distancia medida con una regla entre las dos esquinas exteriores de una fila (`calib_check_mm`); sin regla no se usa salvo que lo permita `calib_allow_unchecked`. El único desplazamiento es el de `camera_offset_x_mm`, `camera_offset_y_mm` y `camera_offset_deg`, y una calibración de imágenes simuladas se rechaza para una cámara real.
* **Una placa puesta en su sitio mirando, moviendo y volviendo a mirar (`src/boardaim.rs`):** se miran los fiduciales desde una pose que no cambia, se calculan el giro y el desplazamiento y se manda a la mesa mover la placa lo contrario (nunca más de `align_max_step_mm` y `align_max_step_deg` por vez, ni `align_max_total_mm` en total); tras esperar imágenes tomadas después del movimiento mira de nuevo, hasta que la placa está dentro de `align_tolerance_mm` y `align_tolerance_deg`. Si algo va mal solo va hacia el estado más seguro (pasos enteros, medios pasos, retener, abortar): un error que crece reduce el paso a la mitad y luego se detiene, y la falta de marcas, imágenes negras o repetidas, un límite o un fallo de la mesa detienen los movimientos. Solo se ha ejecutado con una mesa simulada.

## 🔒 Seguridad y protección

| Amenaza | Defensa |
|---|---|
| Un fichero dañado, cortado o vacío | sin pánico: las pruebas cortan una entrada válida en cada posición y dan basura |
| Un número sin sentido (negativo, no finito, absurdamente grande) | se rechaza con un error que nombra el valor; nada se redondea hasta parecer plausible |
| Un informe o fichero de medidas que escribiría sobre otro fichero | solo se escribe donde no hay ninguno; para escribir sobre uno hace falta `--replace`, y nunca se escribe a través de un enlace |
| Uso inseguro de memoria | Rust seguro y simple con `forbid(unsafe_code)`; un pequeño borde unsafe solo en los motores de cámara compilados con su característica |
| Un token mostrado o enviado donde no debe | el token nunca se imprime ni se pone en un resultado; los valores que envía un servidor se toman todos o ninguno y las claves del enlace (`server_*`) nunca se toman de él; se avisa de un token que iría por HTTP plano a una dirección que no es local |

**Lo que no se defiende:** un resultado producido sin errores no es un resultado verificado. Nada de esto conoce los límites de una máquina y el programa no ha tenido revisión independiente. Informe de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La biblioteca es `eh3d_vision_studio` (`use eh3d_vision_studio::...`); la línea de comandos es `eh3d-vision`. Las órdenes son estas; cada una imprime su resultado por la salida estándar e informa de un error por la salida de errores con un código de salida distinto de cero. Solo la orden `server` usa la red (HTTP plano hacia el servidor de la máquina indicado en `vision.cfg`); todas las demás trabajan con ficheros locales.

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
eh3d-vision train IMAGE --rect X,Y,W,H [--rect X,Y,W,H ...] --output FILE [--replace|--overwrite|--create] [--flatten PX] [--json]   a mark that is none of the known shapes, trained from one or more rectangles cut around it (all of the same size, from train_min_side to train_max_side): their average is kept in a file of a mark (the first line is EH3DMARK 1); a file that exists is never written over unless --replace (--overwrite says the same, --create asks for a new file, the default); exit 2 when the examples do not look like each other, have no contrast, or the mark looks like itself a few pixels away (it could not be told from its neighbours)
eh3d-vision locate --mark FILE IMAGE [--turn DEG] [--min-score S] [--flatten PX] [--json]   the places of the picture that look like a trained mark, with the angle and the score of each (--turn lets the search turn the mark up to DEG each way, 180 is every angle; train_turn_max_deg of vision.cfg when it is not given); exit 2 when the mark is not found, is ambiguous or is found more times than max_parts
eh3d-vision pair DESIGN.csv MEASURED.csv [--any-turn] [--max-turn DEG] [--tolerance MM] [--min-marks N] [--json]   which mark found goes with which mark of the design: by place by default (the same number of marks, the board nearly where it should be), or with --any-turn whatever the turn of the board, up to --max-turn (pair_max_turn_deg of vision.cfg), with a mark missing or an extra one told apart, and the turn, the shift and what is left over; exit 2 when the pairing is ambiguous (a design that looks the same when turned: narrow --max-turn), none is found or fewer marks than --min-marks are paired
eh3d-vision light-check IMAGE [--cells N] [--max-saturated F] [--max-uneven F] [--max-tilt F] [--format text|json|csv] [--output FILE] [--replace]   whether the light of a picture is good to measure from: its mean, the share burnt out, how much the zones differ and the tilt from one side to the other, and why not when it is not (light_* of vision.cfg); exit 2 when the light is not good
eh3d-vision tape-locate IMAGE [--mm-per-px V] [--light] [--pitch-mm MM] [--hole-mm MM] [--format text|json] [--output FILE] [--replace]   the holes along the edge of a tape of parts, the pitch measured against the pitch of the tape, how far the holes lie from their line, the pockets and the one nearest to the centre of the picture, where a part would be picked (tape_* of vision.cfg); exit 2 when there are too few holes, one is missing or off the line, or the pitch or the size is not right
eh3d-vision calib-card [--columns N] [--rows N] [--pitch-mm MM] [--lang CODE] [--output FILE.svg] [--replace]   the card of calibration of round dots (card_* of vision.cfg) as a drawing in millimetres to print at 100 %, with the larger dot at the top left and a bar to measure; a card that does not fit its page is refused, never drawn smaller
eh3d-vision jpeg FRAME [--output FILE.pgm] [--replace] [--json]   the brightness of the picture of a baseline JPEG frame, read within the limits mjpeg_max_bytes and mjpeg_max_pixels; a frame that is cut or wrong is refused with a code that does not change (jpeg_truncated, jpeg_bad_marker, jpeg_bad_table, jpeg_bad_size, jpeg_unsupported, jpeg_bad_data)
eh3d-vision server health|config|discover|check|detect|send KIND ... [--address HOST:PORT] [--machine ID]   the EH3D server of the machine (server_address, server_token, server_machine and server_mdns of vision.cfg): health asks whether it answers, config reads the values of the configuration file that the server keeps for the machine (written as lines for vision.cfg with --output, never over a file without --replace), discover lists the servers that answer on the local network (only with server_mdns = on and a program built with the feature mdns; they are candidates for the person to choose, never connected to by themselves), check FILE tells whether a result follows the convention of the server (schema, kind, picture, findings, units, evidence), detect fiducials|barcode|focus|blobs IMAGE [--seed N] [--artifact] asks the simulated station of the server for a result about the picture (evidence simulator), send quality|offset|pads|diff|orientation|footprint|sharpmap|board takes the arguments of the command of that name and keeps its result as JSON on the server, and send fiducials|focus|blobs IMAGE [--evidence camera|picture|simulator] [--max-findings N] keeps the result in the convention of the server with its schema
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

Las órdenes `train`, `locate`, `pair`, `light-check` y `tape-locate` terminan con el código de salida 0 si se hicieron, 1 si no se pudieron llevar a cabo (una opción mal escrita, un fichero que no se puede leer o escribir, un límite superado) y 2 si se llevaron a cabo pero su comprobación no se superó.

## ⚙️ Configuración

Los valores que se afinan viven en un fichero simple de `clave = valor`, no en el código: `vision.cfg` en la carpeta donde arranca el programa o junto a él (la variable `EH3D_VISION_CONFIG` nombra otro fichero). Fija el idioma de la ventana, si las marcas son oscuras o claras, el umbral, el rango del diámetro, el relleno y la elongación de una marca, el tamaño de la marca que fija la escala, las columnas y filas de la rejilla, si la alineación puede cambiar el tamaño, los límites que debe cumplir una alineación y el mayor fichero que se lee. `examples/vision.cfg` enumera cada clave con su significado y el valor con el que arranca el programa; si falta el fichero se usan esos valores y un fichero con un error se rechaza con su línea. En la línea de comandos lo mismo se da como opciones (véase Interfaces). También guarda los demás valores afinados: la confianza (`refine_subpixel`, `full_contrast`), la ventana de la luz (`flatten_window`), la escala y el tamaño de una pieza (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), el rango de una cruz (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), la tolerancia de un patrón (`pitch_tolerance`) y el máximo de imágenes de una carpeta (`batch_max_files`). El relleno mínimo de un cuadrado y la puntuación mínima de una cruz con cualquier giro son `square_min_fill` y `cross_min_score`. El rango del lado de un cuadrado y cuánto más largo que ancho puede ser su caja son `square_min_size`, `square_max_size` y `square_max_elongation`, aparte de los de la cruz. La lente son `lens_k1` y `lens_k2` (como los calcula `lens`) y lo máximo que una rejilla puede separarse de su retícula tras la corrección es `lens_max_rms`. La tabla de encapsulados que lee `inspect` es `package_table`, y lo máximo que una pieza puede diferir de su encapsulado y girarse son `inspect_tolerance` e `inspect_max_angle`. La puntuación por debajo de la cual el informe HTML avisa de una imagen es `report_low_score`. La distancia entre las marcas del patrón que fija la escala es `pitch_mm`, y `replace_output` dice si los ficheros que se escriben pueden escribirse sobre uno que ya existe (no por defecto). El solapamiento de las imágenes vecinas de un plan de `shots` y el margen alrededor de la pieza son `shots_overlap` y `shots_margin`. Si la ventana dibuja la retícula del objetivo es `show_lens_grid`. El paquete frente al que `inspect` y la ventana juzgan las piezas es `inspect_package`, y el tamaño de la pieza para la que `shots` y la ventana planifican las imágenes es `shots_part_width` y `shots_part_height` (0 y 0: el cuerpo del paquete). Las marcas en anillo tienen el rango de su diámetro exterior (`ring_min_diameter`, `ring_max_diameter`), de la relación del diámetro interior al exterior (`ring_min_ratio`, `ring_max_ratio`), el relleno mínimo de su banda (`ring_min_fill`), la elongación (`ring_max_elongation`), el diámetro que separa los dos tamaños (`ring_split`, 0 para hallarlo) y el hueco entre los tamaños (`ring_size_gap`). El eje de la boquilla en la imagen de una pieza recogida es `nozzle_x_mm` y `nozzle_y_mm`, y lo más lejos que puede sostenerse y lo más que puede girar son `pick_max_offset_mm` y `pick_max_turn_deg` (0: no se comprueba). El giro de una pieza bien sostenida es `nominal_angle_deg`, si la cámara ve las piezas en espejo es `pick_mirror`, y la comprobación de la imagen es `quality_check` con los mínimos `quality_min_contrast`, `quality_min_mean`, `quality_max_mean` y `quality_min_sharpness` (0: no se comprueba). El servidor son `server_address`, `server_token`, `server_machine`, `server_timeout_s` y `server_retries` (dirección vacía: no hay servidor); la búsqueda por correlación es `correlate_diameter` y `correlate_min_score`; el lado mayor de una imagen corregida es `rectify_max_side`; la tolerancia de una fila de pads es `pads_tolerance`; y la comparación de dos imágenes es `diff_threshold`, `diff_min_area` y `diff_max_changed`. Si el servidor se busca en la red local lo dice `server_mdns` (apagado por defecto). La lectura de una pieza con marca es `orientation_body_contrast`, `orientation_mark_contrast`, `orientation_min_mark_px` y `orientation_tolerance_deg`; la tolerancia de un pad es `footprint_tolerance_mm`; el mapa de nitidez es `sharpmap_cells`, `sharpmap_min_share` y `sharpmap_min_contrast`; y la escala desde un tablero es `board_square_mm` (la distancia entre centros es `pitch_mm`), `board_max_aspect` y `board_max_spread`. Un `vision.cfg` que tenga un token debe mantenerse privado. La nitidez que cuenta como 50 puntos en un resultado de enfoque es `focus_reference`, y el paso a lo largo de la línea de un borde y la mayor incertidumbre de su posición son `edge_step_px` y `edge_max_uncertainty_px`. La cámara en vivo, el tablero de calibración y la alineación de una placa tienen sus propias claves (`capture_*`, `calib_*`, `camera_offset_*` y `align_*`): cada una tiene un rango, un valor fuera de él se rechaza y un error que una clave gobierna nombra la clave. El lector de fotogramas JPEG en movimiento, la tarjeta de puntos, la comprobación de la luz y la búsqueda de los agujeros de una cinta tienen sus propias claves (`mjpeg_*`, `card_*`, `light_*` y `tape_*`): cada una tiene un rango, y un valor fuera de él se rechaza nombrando la clave.

## 🚀 Inicio rápido

Necesita una cadena de herramientas Rust estable (1.88 o más reciente). El repositorio EH3D-COMMON debe estar clonado junto a este: el lector de imágenes se toma de él por ruta.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Para sacar una captura de la ventana sin abrirla a mano, ejecútelo con `--shot FICHERO.png` (también `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`): se abre con ese tamaño, guarda el PNG, no guarda ajustes y se cierra solo (código de salida 0 si se escribió el fichero, 1 si no, 2 si los argumentos están mal). Con `--mark N` la marca número N (desde 1) fija la escala y con `--align` el diseño se alinea con las marcas, de modo que la captura muestra las líneas de la alineación: desde el lugar donde el diseño pone cada marca hasta la marca encontrada, con la distancia en milímetros (`show_residuals` en `vision.cfg` las desactiva). `examples/design_pair.csv` es un diseño para las dos marcas de `examples/marks.pgm`.

## 🧪 Pruebas

El conjunto tiene 967 pruebas (`cargo test`, ejecutadas en Windows y Linux en el CI). Comprueban los casos normales, los casos límite y las entradas dañadas: ficheros vacíos, basura y ficheros cortados nunca provocan un pánico. El CI ejecuta además `cargo fmt --check`, `cargo clippy` con las advertencias como errores y `python -I tools/ci_validate.py`.

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
│   ├── batch.rs                                 una carpeta de imágenes de una vez, con un informe CSV
│   ├── board.rs                                 la escala desde un tablero de cuadrados: a lo largo de x, de y y desde el lado, con informe
│   ├── boardaim.rs                              una placa puesta donde dice su diseño: mirar, mover, volver a mirar, siempre hacia lo más seguro
│   ├── capture/                                 la cámara en vivo: trait, formatos, propiedades, presupuesto USB, estado del flujo, simulada
│   ├── charuco/                                 el tablero de calibración con marcadores numerados: esquinas, calibración, fichero y tarjeta
│   ├── config.rs                                el fichero de configuración: cada valor afinado
│   ├── correlate.rs                             marcas redondas halladas por correlación con un disco del diámetro buscado
│   ├── diff.rs                                  dos imágenes del mismo sitio comparadas: qué cambió, dónde y cuánto
│   ├── discovery.rs                             respuestas de la red como lista corta de candidatos seguros, y un resolutor guionizado
│   ├── cross.rs                                 marcas fiduciales en forma de cruz
│   ├── edge.rs                                  un borde con fracción de píxel a lo largo de una línea, con su incertidumbre
│   ├── fiducial.rs                              las marcas redondas de una imagen: umbral, manchas y comprobaciones
│   ├── flatten.rs                               la luz desigual quitada de una imagen
│   ├── footprint.rs                             cada pad juzgado frente al tamaño del diseño, con una tolerancia en milímetros
│   ├── grid.rs                                  la distorsión del objetivo a partir de una rejilla de marcas
│   ├── inspect.rs                               cada pieza comparada con el cuerpo de su encapsulado desde una tabla, con veredicto
│   ├── job.rs                                   el informe de una carpeta leído en un hilo, con progreso y cancelación
│   ├── lens.rs                                  los dos factores de la lente desde una rejilla de marcas, rechazados si no encajan
│   ├── points.rs                                los ficheros de puntos de un diseño
│   ├── quality.rs                               si una imagen basta para medir: su nitidez, su contraste y su nivel medio
│   ├── reference.rs                             una imagen contra una referencia con una máscara de tolerancia
│   ├── histogram.rs                             el histograma y los cortes de Otsu, isodata y el método del triángulo
│   ├── report.rs                                el informe de una carpeta como una página HTML autónoma, en ocho idiomas
│   ├── ring.rs                                  las marcas en anillo: redondas con un agujero, de uno o dos tamaños, con confianza
│   ├── limit.rs                                 lectura de un fichero con límite de tamaño y un código para cada rechazo
│   ├── measure.rs                               piezas medidas en milímetros, ficheros CSV y JSON
│   ├── offset.rs                                dónde se sostiene una pieza recogida, con la corrección, la tabla de una bandeja y el eje de la boquilla
│   ├── orientation.rs                           en qué sentido está sujeta una pieza con marca, por la dirección de la marca
│   ├── pads.rs                                  una fila de pads: el paso, cuánto varía y cuánto se aparta un pad de la fila
│   ├── panels.rs                                los paneles de la ventana: objetivo, paquetes, plan de imágenes, desplazamientos y comprobación
│   ├── perspective.rs                           perspectiva corregida: la imagen dibujada de nuevo vista desde arriba, desde cuatro esquinas
│   ├── scale.rs                                 de píxeles a milímetros y la distorsión radial del objetivo
│   ├── server.rs                                el camino al servidor EH3D por el cliente compartido y un servidor en memoria para pruebas
│   ├── convention.rs                            el resultado de visión en la convención del servidor: crearlo, comprobarlo, SHA-256
│   ├── station.rs                               la estación de visión simulada del servidor: el trabajo, su respuesta, un port
│   ├── sharpmap.rs                              un mapa de dónde es nítida la imagen: una rejilla de celdas, las débiles y la inclinación
│   ├── square.rs                                marcas cuadradas y cruces girados con cualquier ángulo
│   ├── tools.rs                                 las herramientas del panel lateral sobre la imagen: formas, piezas, informe y paso
│   ├── train.rs                                 una marca entrenada con ejemplos, hallada por correlación en el ángulo en que está girada
│   ├── turnpair.rs                              marcas emparejadas con el diseño con la placa girada a cualquier ángulo
│   ├── dotcard.rs                               la tarjeta de puntos redondos para imprimir al 100 %, como dibujo en milímetros
│   ├── light.rs                                 si la luz de una imagen es buena: media, quemado, zonas e inclinación
│   ├── mjpeg.rs                                 un lector de fotogramas JPEG en movimiento que no se fía de sus bytes
│   ├── tape.rs                                  los agujeros y bolsillos de una cinta de piezas y su paso frente al de la cinta
│   ├── verbs.rs                                 las órdenes train, locate, pair, light-check, tape-locate, calib-card y jpeg
│   ├── shot.rs                                  el modo de captura de la ventana (--shot): sus argumentos, leídos y comprobados sin pantalla
│   ├── shots.rs                                 los lugares de la cámara que cubren una pieza mayor que su campo, solo con los tamaños
│   ├── browse.rs                                la lista de ficheros de la ventana que abre un fichero
│   ├── checks.rs                                paneles de la ventana: correlación, enderezado, fila de pads y comparación de imágenes
│   ├── readings.rs                              paneles de la ventana: histograma, borde, referencia y ajustes del servidor
│   ├── gui.rs                                   la ventana (egui, con la característica gui): dibuja el estado y transmite lo que hace la persona
│   ├── lang.rs                                  las palabras de la ventana en los ocho idiomas
│   ├── viewport.rs                              zoom y desplazamiento de la imagen, en números simples
│   └── window.rs                                el estado de la ventana sin pantalla: imagen, marcas, escala, objetivo y alineación
├── tests/
│   ├── readme.rs                                pruebas que ejecutan lo que prometen el README y los scripts run
│   ├── strict.rs                                pruebas de la línea de comandos: opciones y palabras que no conoce
│   ├── read_limit.rs                            pruebas del límite de tamaño de los ficheros que lee la línea de comandos
│   ├── exchange.rs                              pruebas de las órdenes que leen una imagen y escriben un fichero
│   ├── commands.rs                              pruebas de las órdenes correlate, rectify, pads, diff y server
│   ├── analysis.rs                              pruebas de las órdenes orientation, footprint, sharpmap y board
│   ├── capture.rs                               cámara en vivo, calibración y alineación con la cámara simulada, y el comando
│   ├── fuzz.rs                                  pruebas con entrada aleatoria y semilla fija: nada debe entrar en pánico
│   ├── camera.rs                                las órdenes nuevas ejecutadas como programa: edge, histogram, refcheck, residuos, server
│   ├── fuzz_camera.rs                           entrada aleatoria con semilla fija para las lecturas nuevas y la convención
│   ├── fuzz_capture.rs                          entrada aleatoria con semilla fija para lectores de imagen, calibración, alineación
│   ├── training.rs                              una marca entrenada hallada donde y como se dibujó, y sus ficheros
│   ├── pairing.rs                               marcas de una placa girada a cualquier ángulo emparejadas según la verdad
│   ├── mjpeg.rs                                 fotogramas codificados leídos, cortados en todo sitio, bytes cambiados, ruido
│   ├── newcli.rs                                las órdenes nuevas: texto, JSON, ficheros respetados, códigos de salida, ejemplos
│   ├── lightfuzz.rs                             luz, cinta y tarjeta con cualquier imagen y ajuste, con semilla fija
│   ├── discovery.rs                             las respuestas de la red, hostiles o muchas, nunca dan un candidato inseguro
│   ├── shot_sample.rs                           pruebas de que las muestras se abren en la ventana de una captura
│   ├── vision.rs                                pruebas de la biblioteca y la línea de comandos
│   └── reqs.rs                                  pruebas del registro de requisitos
├── examples/
│   ├── design.csv                               una muestra para probar el programa
│   ├── design_pair.csv                          un diseño para las dos marcas de marks.pgm
│   ├── marks.pgm                                una muestra para probar el programa
│   ├── measured.csv                             una muestra para probar el programa
│   ├── packages.csv                             una tabla de cuerpos de encapsulados para inspect
│   └── vision.cfg                               el fichero de configuración con cada valor y su significado
├── docs/
│   ├── architecture.json                        datos de la imagen de Arquitectura
│   ├── NOT_YET.md                               buenas ideas fuera de esta versión, con el porqué y el cuándo
│   ├── REQUIREMENTS.md                          el registro de requisitos: estado, evidencia y la prueba de cada uno
│   ├── SCOPE.md                                 para qué sirve, qué no hará y qué no decide nunca
│   ├── STATUS.md                                qué existe, qué falta y qué sigue abierto
│   └── structure.json                           datos de la imagen de Estructura
├── tools/                                       comprobación del CI y del README y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el workspace y sus dependencias fijadas
├── rust-toolchain.toml                          la toolchain de Rust fijada
├── CHANGELOG.md                                 qué cambió en cada versión
├── AGENTS.md                                    las reglas para quien cambie el repositorio
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* La ventana todavía no se ha visto en una pantalla; `align` y el bucle de mirar, mover y volver a mirar emparejan las marcas con el diseño por su posición, así que allí una placa muy girada o una marca que falta se rechaza (`pair --any-turn` sí las empareja).
* No se ha probado en una máquina ni con una colección de ficheros reales y no ha tenido revisión independiente.
* Lee ficheros de imagen, y una cámara en vivo solo en la simulación (los motores de Windows, V4L2 y libcamera son esqueletos que no se han probado con una cámara real); solo encuentra marcas redondas, cuadradas y en forma de cruz y se ha probado con imágenes dibujadas por las pruebas, no con fotografías de placas reales. El giro de una pieza se mide con sus píxeles como el de un rectángulo, así que una pieza que no es un rectángulo recibe el giro de su eje largo.
* El servidor no tiene complemento de visión, así que el programa usa los artefactos del almacén (`POST /api/v1/artifacts` para un resultado, `GET /api/v1/artifacts` y `GET /api/v1/artifacts/{id}` para la configuración de una máquina) con nombres y un formato propios (`vision-<kind>.json`, `vision-<machine>.cfg`); hoy solo se prueba contra un servidor en memoria y un socket en el bucle local, y la búsqueda del servidor por mDNS (`server_mdns`) es un interruptor sin resolutor enlazado.

**Siguiente**

* Leer la cámara en vivo de la máquina (EH3D-NODE-BUNGARD-AUX), probar fotografías de placas reales y poner en la ventana el entrenamiento de marcas nuevas, que las órdenes ya tienen.
* Ver la ventana en una pantalla y terminarla.
* Probarlo con ficheros reales y con los demás programas de la suite.
* Escribir los motores en vivo (imágenes de Media Foundation, V4L2 y libcamera), probar el tablero de calibración con una lente real, la tarjeta de puntos en papel y la alineación con una mesa real.

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
* [Alcance](docs/SCOPE.md): para qué sirve el programa, qué no hará y qué no decide nunca.
* [Requisitos](docs/REQUIREMENTS.md): el registro de capacidades, el estado de cada una y la prueba que la muestra.
* [Estado](docs/STATUS.md): qué existe, qué falta y qué sigue abierto.
* [Todavía no](docs/NOT_YET.md): buenas ideas que quedan fuera de esta versión.
* [Reglas para cambiar el repositorio](AGENTS.md): las reglas para quien cambie el repositorio.
* [Datos de arquitectura y estructura](docs/architecture.json): los datos con los que se dibujan las dos imágenes.

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `python -I tools/ci_validate.py`: todo debe pasar. Véanse [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio de texto se hace en los ocho ficheros, y las dos imágenes se dibujan desde `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Informe de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
