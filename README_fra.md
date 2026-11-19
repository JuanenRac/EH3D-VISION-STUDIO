<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciaires dans une image de caméra : marques, échelle et correction d'une carte

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.7-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`eh3d-vision`) avec 300 tests exécutés sur un ordinateur : trouve des marques fiduciaires rondes dans une image, convertit les pixels en millimètres et calcule la rotation et le décalage qui placent une carte où son dessin le dit. Il a une fenêtre (compilée avec `--features gui`, le programme `eh3d-vision-gui`) qui ouvre une image, montre les marques avec leurs coordonnées, fixe l'échelle, ajuste la distorsion de l'objectif avec une grille, aligne sur le dessin et exporte les options d'ISOCAM, mais la fenêtre n'a pas encore été vue sur un écran ; il n'a pas été essayé sur une machine et n'a pas été vérifié sur une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.7` |
| Maturité | Fonctionnel |
| Tests | 300 |
| Pile technique | Rust |
| Sur du matériel réel | Non essayé : rien ici n'a fonctionné sur une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Cœur | 🟡 | trouve des marques fiduciaires rondes dans une image, convertit les pixels en millimètres et calcule la rotation et le décalage qui placent une carte où son dessin le dit ; testé sur un ordinateur avec des entrées normales et abîmées, pas avec une collection de fichiers réels. |
| Ligne de commande | 🟡 | `eh3d-vision` exécute les mêmes fonctions que la bibliothèque et affiche le résultat ; chaque erreur revient sous forme de message et de code de sortie non nul. |
| Tests | 🟡 | 300 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées, et une caméra simulée au-dessus d'une carte qui ajoute du bruit, une mauvaise exposition, une image morte ou absente, une poussière qui ressemble à un repère ou un fichier d'image coupé, et qu'on interroge de nouveau. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🟠 | Écrite (egui, `--features gui`, le programme `eh3d-vision-gui`) : ouvrir ou déposer une image, voir les marques avec leurs coordonnées, fixer l'échelle avec une marque de taille connue, ajuster la distorsion de l'objectif avec une grille, aligner sur un dessin CSV et exporter les options d'ISOCAM ; sa logique est testée, mais elle n'a pas encore été compilée ni vue sur un écran. |
| Matériel réel | 🔴 | Non essayé : rien ici n'a été exécuté sur une machine. |
| Documentation | 🟡 | README en huit langues, le journal des modifications et les deux images dessinées depuis `docs/`. |

## 🎯 Présentation

* **Les marques** (`src/fiducial.rs`) : l'image est coupée en sombre et clair avec le seuil d'Otsu, les taches connexes sont mesurées, et celles qui sont des disques de la taille attendue (la surface, le remplissage du cercle qui les contient et leur allongement) sont les marques, les meilleures d'abord ; une tache coupée par le bord de l'image est écartée car son centre n'est pas fiable, et les marques peuvent être sombres sur clair ou claires sur sombre.
* **L'échelle et l'objectif** (`src/scale.rs`) : les millimètres d'un pixel viennent d'une marque de taille connue, les axes de la machine montent tandis que l'image descend, et la distorsion radiale de l'objectif est retrouvée par moindres carrés à partir d'une grille de marques puis corrigée.
* **La carte** (`src/align.rs`) : à partir de deux fiduciaires ou plus du dessin et des mêmes tels que la caméra les a trouvés, la rotation, le décalage et (si permis) la taille sont calculés par moindres carrés, avec le résidu en millimètres ; une transformation au résidu trop grand (une mauvaise marque) ou à la taille déviée (une carte étirée) n'est pas acceptée.

## 🧩 Rôle dans EH3D

EH3D-VISION-STUDIO est un programme de l'EH3D Fabrication Suite. Il fonctionne comme bibliothèque et ligne de commande et, compilé avec `--features gui`, comme fenêtre avec l'aspect commun aux autres programmes de la suite. Il lit et écrit des fichiers simples et peut donc s'utiliser seul ou entre les autres programmes.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-VISION-STUDIO : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>


Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée depuis `docs/architecture.json` et vérifiée avec le code.

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `eh3d-vision` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Ligne de commande stricte :** une option mal écrite (`--powr`) ou un mot en trop est une erreur qui la nomme, et non quelque chose d'ignoré ; sinon le programme ferait sa sortie avec les valeurs par défaut.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Limite de taille des fichiers lus :** la ligne de commande ne lit une image ou un fichier de points que s'il n'est pas plus grand que `max_file_mib` de `vision.cfg` (64 Mio par défaut) ; un fichier plus grand est refusé avec le code `file_too_large` et son nom, un fichier illisible avec `file_unreadable` et un fichier de points qui n'est pas du texte avec `file_not_text`, et rien n'est chargé.
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.
* **Les repères pour ISOCAM :** `align --isocam-args` écrit chaque paire de repères comme l'option `--fiducial X_CONCEPTION,Y_CONCEPTION,X_MESURE,Y_MESURE` de `eh3d-isocam`.
* **Une fenêtre :** compilée avec `--features gui` comme le programme `eh3d-vision-gui` (sous Windows elle s'ouvre sans console) : elle ouvre une image PNG ou PGM avec zoom et déplacement, trouve les marques avec des valeurs modifiables, les dessine avec leurs coordonnées, fixe l'échelle avec une marque de taille connue, ajuste la distorsion de l'objectif avec une grille de marques, aligne sur un dessin d'un fichier CSV (les marques lui sont appariées selon leur position, donc l'ordre n'importe pas) et exporte les options `--fiducial` d'ISOCAM seulement quand l'alignement est accepté. Elle parle les huit langues et n'a pas besoin de réseau.
* **Un fichier de configuration :** `vision.cfg` contient chaque valeur réglée (l'aspect d'une marque, la grille de la mire, les limites d'un alignement accepté, le plus gros fichier lu) ; une erreur dedans est refusée avec sa ligne, et `examples/vision.cfg` les énumère toutes.
* **Le centre à une fraction de pixel, avec une confiance :** `find --refine` (ou `refine_subpixel` dans `vision.cfg`) pondère les pixels gris du bord de chaque marque, si bien que son centre est trouvé à une fraction de pixel, et affiche une confiance qui baisse quand la marque ressort à peine de la carte.
* **Lumière inégale :** `--flatten PX` (ou `flatten_window`) divise l'image par la lumière autour de chaque pixel avant la recherche, pour qu'une lampe qui faiblit sur la carte ou une ombre ne cache pas les marques du côté sombre.
* **Marques en forme de croix :** `eh3d-vision crosses IMAGE` trouve des signes plus dont les bras suivent les axes de l'image (boîte carrée, deux bras de même largeur, coins vides) ; les disques et les carrés sont écartés, et `--turned` trouve les croix à n'importe quel angle (voir ci-dessous).
* **L'échelle à partir du pas d'une mire :** `eh3d-vision calibrate IMAGE --pitch-mm D` prend la médiane des distances entre marques voisines d'une grille, refuse une mire qui n'est pas vue de face et affiche la ligne `mm_per_px = ...` pour `vision.cfg`.
* **Des pièces pour d'autres programmes, en fichiers simples :** `eh3d-vision measure IMAGE --mm-per-px V` donne pour chaque pièce le centre (mm depuis le centre de l'image, x à droite, y vers le haut), l'angle (de -90 exclu à 90 degrés), la longueur, la largeur et l'aire, en CSV (`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`) ou en JSON (`--format json` : `format` `eh3d-vision-parts`, `version` 1, `unit`, `image_width_px`, `image_height_px`, `mm_per_px`, `parts`) ; rien d'autre n'est partagé avec le programme qui les lit. `--output FICHIER` n'écrit jamais sur un fichier sans `--replace`, ni sur un lien.
* **Un dossier d'un coup :** `eh3d-vision batch DOSSIER --report FICHIER.csv` lit chaque image PNG et PGM dans la limite de taille (au plus `batch_max_files`) et écrit une ligne par image (`file,status,marks,mean_score,min_score,mean_diameter_px,detail`) avec l'état `ok`, `no_marks`, `picture_unreadable` ou le code du refus ; un nom de fichier qu'un tableur exécuterait comme une formule est écrit avec une apostrophe devant.
* **Carrés et croix à n'importe quel angle :** `eh3d-vision squares IMAGE` trouve des marques carrées tournées d'un angle quelconque (centre, côté et angle anti-horaire tel que la machine le voit, de -45 exclu à 45 degrés) et `eh3d-vision crosses IMAGE --turned` trouve des croix dont les bras ne suivent pas les axes ; le remplissage minimal d'un carré et le score minimal d'une croix (`square_min_fill`, `cross_min_score`) sont dans `vision.cfg`.
* **Repères en anneau d'une taille ou de deux :** `eh3d-vision rings IMAGE` trouve des repères ronds percés (donuts) : chacun a son centre, son diamètre extérieur et intérieur, le score de sa forme, une confiance (le score abaissé quand l'anneau ressort peu du fond, `full_contrast`) et, quand les anneaux sont de deux tailles, s'il est petit ou grand (`--split PX` ou `ring_split` donne le diamètre qui les sépare, sinon il est trouvé là où les diamètres forment deux groupes séparés d'au moins `ring_size_gap`). Un disque, un anneau avec une coupure, un anneau coupé par le bord, un anneau pas rond et un anneau dont le trou est trop petit ou trop grand (`ring_min_ratio`, `ring_max_ratio`) sont écartés. La fenêtre a une section pour eux qui dessine le cercle extérieur et l'intérieur d'une couleur par taille.
* **Où une pièce prise est tenue :** `eh3d-vision offset IMAGE --package NOM` prend l'image d'une pièce vue d'en dessous (une seule pièce), la mesure comme `measure`, compare sa taille au corps de son boîtier comme `inspect` et donne son décalage par rapport à l'axe de la buse (`--nozzle-x`, `--nozzle-y` ou `nozzle_x_mm`, `nozzle_y_mm`, millimètres depuis le centre de l'image, x vers la droite et y vers le haut), sa rotation par rapport à la façon dont elle devrait être tenue (`--nominal-angle` ; une pièce aussi longue que large tourne par le plus court dans un quart de tour, toute autre dans un demi-tour) et la correction qui la met où la buse la voudrait : les mêmes distances et la rotation en sens inverse. CSV ou JSON, jamais écrit sur un fichier sans `--replace` ; le code de sortie est 1 quand la pièce a la mauvaise taille, est tenue plus loin que `--max-offset` (`pick_max_offset_mm`) ou tournée de plus de `--max-turn` (`pick_max_turn_deg`), une fois le fichier écrit, et une image sans pièce ou avec plus d'une pièce est refusée (`no_parts`, `parts_count`). C'est de la géométrie sur une image : ni caméra, ni buse, ni machine n'ont servi.
* **Une table de corrections pour un plateau et un panneau de la fenêtre pour elle :** `eh3d-vision offset IMAGE --package NOM --each` prend chaque pièce de l'image (un plateau), chacune à part, et écrit une table avec la colonne `part` (le numéro de la pièce, à partir de 1) puis les colonnes de `offset`, une ligne par pièce ; le code de sortie est 1 quand une pièce échoue. La fenêtre a un panneau pour la même chose (`nozzle_x_mm`, `nozzle_y_mm`, `nominal_angle_deg`, `pick_max_offset_mm`, `pick_max_turn_deg`) : il calcule la table de l'image ouverte, liste la correction de chaque pièce dans la langue de la fenêtre et l'enregistre sous `<image>-offsets.csv`. `nominal_angle_deg` est la rotation d'une pièce bien tenue, aussi dans `vision.cfg`.
* **Une caméra en miroir et l'axe de la buse :** `--mirror` (`pick_mirror`) sert pour une caméra qui voit les pièces retournées de gauche à droite : le x et la rotation de chaque pièce changent de signe. `eh3d-vision nozzle IMAGE IMAGE... --package NOM` prend des images de pièces prises avec la même buse (une pièce dans chacune, au moins deux images) et écrit les lignes `nozzle_x_mm` et `nozzle_y_mm` pour `vision.cfg`, la place moyenne des pièces (elle ne dépend pas de l'axe utilisé lors de la mesure) ; sur la sortie d'erreur, elle indique la dispersion des pièces (l'écart type en x et en y, la pièce la plus éloignée et la rotation moyenne).
* **L'image suffit-elle pour mesurer :** `eh3d-vision quality IMAGE` donne la netteté (la variance du laplacien), le contraste (l'écart type des niveaux de gris) et le niveau de gris moyen d'une image, en CSV ou JSON, et la refuse (code de sortie 1, après l'écriture du fichier) comme `flat`, `dark`, `bright` ou, quand `--min-sharpness` dépasse 0, `blurry` ; les minimums sont `--min-contrast`, `--min-mean`, `--max-mean` et `--min-sharpness`, ou les clés de `vision.cfg`. Avec `quality_check = yes`, `offset`, `nozzle` et la fenêtre refusent une mauvaise image (`poor_picture`) avant d'y mesurer une pièce, et la fenêtre a un panneau pour contrôler l'image et montrer ses chiffres. Les valeurs dépendent de la caméra et de l'objectif, elles sont donc dans le fichier de configuration et la netteté commence désactivée ; rien de cela n'a été essayé avec une vraie caméra.
* **L'objectif à deux facteurs, refusé s'il ne colle pas :** `eh3d-vision lens IMAGE` calcule `k1` et `k2` de la distorsion radiale à partir d'une grille de marques et écrit les lignes `lens_k1 = ...` et `lens_k2 = ...` pour `vision.cfg` ; il refuse un ajustement qui laisse les marques à plus de `lens_max_rms` pixels de leur réseau (`lens_residual_too_large`) ou un facteur supérieur à 0,5 (`lens_out_of_range`), et `find --undistort` redresse les positions avec eux.
* **Un verdict pour chaque pièce :** `eh3d-vision inspect IMAGE --package NOM --table packages.csv` mesure les pièces d'une image et compare chacune au corps de son boîtier (`package`, `body_long_mm`, `body_short_mm` et, si besoin, `tolerance_mm`, dans un fichier CSV ; `examples/packages.csv` en contient quelques-uns) : `pass` ou `fail` avec la raison (`length`, `width`, `angle`) en CSV ou JSON, le code de sortie est 1 quand une pièce échoue ; la tolérance et la rotation maximale (`inspect_tolerance`, `inspect_max_angle`) sont dans `vision.cfg`.
* **Un rapport HTML du dossier :** `eh3d-vision batch DOSSIER --html rapport.html` écrit une page autonome (sans script, sans lien, sans autre fichier) : un tableau des images, un petit dessin de chacune avec un cercle sur chaque marque trouvée et les avertissements (une image inutilisable, sans marques ou avec un score sous `report_low_score`), dans la langue de `--lang` ou de `vision.cfg`.
* **Les nouveaux outils dans la fenêtre :** le panneau latéral propose le centre à une fraction de pixel avec la confiance de chaque marque, la fenêtre de la lumière, les carrés et les croix à n'importe quel angle (dessinés sur l'image), les pièces en millimètres (enregistrées sous `<image>-parts.csv`), le rapport du dossier, lu dans un fil avec barre de progression et bouton d'annulation (`eh3d-vision-report.html`, jamais écrit sur un fichier existant sans `replace_output`) et l'échelle depuis le pas d'une mire, avec les textes dans les huit langues et les valeurs dans `vision.cfg` ; elle n'a pas encore été regardée sur un écran.
* **L'objectif dans la fenêtre, avec les deux facteurs :** le bouton de la grille ajuste `k1` et `k2` comme le fait `eh3d-vision lens` (refusé au-dessus de `lens_max_rms`, un champ du panneau) ; la fenêtre dessine les endroits où l'objectif ajusté place le réseau de la grille, à comparer avec les marques trouvées (`show_lens_grid`), prend les facteurs de `lens_k1` et `lens_k2` de `vision.cfg` sans grille et les enregistre sous `<image>-lens.cfg`.
* **Pièces face à leur boîtier et images d'une grande pièce, dans la fenêtre :** le panneau latéral lit la table des boîtiers (`package_table`), juge les pièces mesurées face au boîtier `inspect_package` (ligne verte ou rouge par pièce, avec la raison de l'échec ; `inspect_tolerance` et `inspect_max_angle` sont des champs du panneau) et enregistre les verdicts sous `<image>-inspection.csv`, le fichier de `eh3d-vision inspect` ; il planifie aussi les images qu'une pièce plus grande que le champ demande (le champ est l'image en millimètres, la pièce est `shots_part_width` par `shots_part_height` ou le corps du boîtier, avec `shots_overlap` et `shots_margin`), liste où va la caméra et enregistre `<image>-shots.csv`, le fichier de `eh3d-vision shots`. La ligne de commande prend les deux mêmes nouvelles clés quand ni `--package` ni la taille de la pièce ne sont donnés.
* **Les images d'une grande pièce, planifiées :** `eh3d-vision shots --package NOM --fov-w MM --fov-h MM` (ou `--body-long` et `--body-short`) liste en CSV les positions de la caméra, en millimètres depuis le centre de la pièce, qui voient toute une pièce plus grande que le champ, avec des images voisines qui se recouvrent de `shots_overlap` et une marge de `shots_margin` ; aucune caméra n'est nécessaire.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Un rapport ou un fichier de mesures qui écraserait un autre fichier | on n'écrit que là où il n'y en a pas ; il faut `--replace` pour écraser, et on n'écrit jamais à travers un lien |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_vision_studio` (`use eh3d_vision_studio::...`) ; la ligne de commande est `eh3d-vision`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice
eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
eh3d-vision rings IMAGE [--light] [--min-size PX] [--max-size PX] [--split PX] [--flatten PX]   the ring marks found (donuts: a round mark with a hole), with the outer and inner diameter, the size class when the rings are of two sizes (--split gives the diameter that tells small from large, else it is found from the diameters), the score of the shape and a confidence
eh3d-vision offset IMAGE --package NAME [--table FILE] [--mm-per-px V] [--nozzle-x MM] [--nozzle-y MM] [--nominal-angle DEG] [--tolerance MM] [--max-offset MM] [--max-turn DEG] [--mirror] [--each] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where a picked part is held, from a picture of it taken from below (one part in it): its offset from the axis of the nozzle and its turn from the way it should be held, the correction that puts it right, its size against the body of the package; --mirror when the camera sees the part turned over left to right; --each takes every part of the picture (a tray) and writes a table of corrections, one row for each; exit 1 when a part is the wrong size, too far off or turned too much
eh3d-vision nozzle IMAGE IMAGE... --package NAME [--table FILE] [--mm-per-px V] [--mirror] [--light] [--flatten PX] [--output FILE] [--replace]   where the axis of the nozzle is, from pictures of parts picked with it (one part in each, at least two pictures): the lines nozzle_x_mm and nozzle_y_mm for vision.cfg, with how spread the parts are
eh3d-vision quality IMAGE [--min-contrast V] [--min-mean V] [--max-mean V] [--min-sharpness V] [--format csv|json] [--output FILE] [--replace]   whether a picture is good enough to measure from: its sharpness, contrast and mean grey level, and why it is refused (flat, dark, bright or blurry); exit 1 when it is refused
eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuration

Les valeurs réglées se trouvent dans un simple fichier `clé = valeur`, pas dans le code : `vision.cfg` dans le dossier où le programme démarre ou à côté de lui (la variable `EH3D_VISION_CONFIG` désigne un autre fichier). Il fixe la langue de la fenêtre, si les marques sont sombres ou claires, le seuil, la plage du diamètre, le remplissage et l'allongement d'une marque, la taille de la marque qui fixe l'échelle, les colonnes et les lignes de la grille, si l'alignement peut changer la taille, les limites qu'un alignement doit respecter et le plus gros fichier lu. `examples/vision.cfg` énumère chaque clé avec sa signification et la valeur avec laquelle le programme démarre ; sans fichier, ces valeurs s'appliquent, et un fichier avec une erreur est refusé avec sa ligne. En ligne de commande les mêmes choses se donnent comme options (voir Interfaces). Il contient aussi les autres valeurs réglées : la confiance (`refine_subpixel`, `full_contrast`), la fenêtre de la lumière (`flatten_window`), l'échelle et la taille d'une pièce (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), l'étendue d'une croix (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), la tolérance d'une mire (`pitch_tolerance`) et le nombre maximal d'images d'un dossier (`batch_max_files`). Le remplissage minimal d'un carré et le score minimal d'une croix à n'importe quel angle sont `square_min_fill` et `cross_min_score`. La plage du côté d'un carré et de combien sa boîte peut être plus longue que large sont `square_min_size`, `square_max_size` et `square_max_elongation`, à part celles de la croix. L'objectif est `lens_k1` et `lens_k2` (tels que `lens` les calcule) et l'écart maximal d'une grille à son réseau après la correction est `lens_max_rms`. La table des boîtiers que lit `inspect` est `package_table`, et l'écart maximal d'une pièce à son boîtier et sa rotation maximale sont `inspect_tolerance` et `inspect_max_angle`. Le score sous lequel le rapport HTML avertit à propos d'une image est `report_low_score`. La distance entre les marques de la mire qui fixe l'échelle est `pitch_mm`, et `replace_output` dit si les fichiers écrits peuvent l'être sur un fichier existant (non par défaut). Le recouvrement des images voisines d'un plan de `shots` et la marge autour de la pièce sont `shots_overlap` et `shots_margin`. Que la fenêtre dessine ou non le réseau de l'objectif se règle avec `show_lens_grid`. Le boîtier face auquel `inspect` et la fenêtre jugent les pièces est `inspect_package`, et la taille de la pièce pour laquelle `shots` et la fenêtre planifient les images est `shots_part_width` et `shots_part_height` (0 et 0 : le corps du boîtier). Les repères en anneau ont la plage de leur diamètre extérieur (`ring_min_diameter`, `ring_max_diameter`), du rapport du diamètre intérieur à l'extérieur (`ring_min_ratio`, `ring_max_ratio`), le remplissage minimal de leur bande (`ring_min_fill`), l'allongement (`ring_max_elongation`), le diamètre qui sépare les deux tailles (`ring_split`, 0 pour le trouver) et l'écart entre les tailles (`ring_size_gap`). L'axe de la buse dans l'image d'une pièce prise est `nozzle_x_mm` et `nozzle_y_mm`, et la distance maximale où elle peut être tenue et la rotation maximale sont `pick_max_offset_mm` et `pick_max_turn_deg` (0 : non vérifié). La rotation d'une pièce bien tenue est `nominal_angle_deg`, le fait que la caméra voie les pièces en miroir est `pick_mirror`, et le contrôle de l'image est `quality_check` avec les minimums `quality_min_contrast`, `quality_min_mean`, `quality_max_mean` et `quality_min_sharpness` (0 : non vérifié).

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.88 ou plus récente). Le dépôt EH3D-COMMON doit être cloné à côté de celui-ci : le lecteur d'images en est pris par chemin.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Pour prendre une capture de la fenêtre sans l'ouvrir à la main, lancez-la avec `--shot FICHIER.png` (aussi `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`) : elle s'ouvre à cette taille, enregistre le PNG, ne garde aucun réglage et se ferme toute seule (code de sortie 0 si le fichier a été écrit, 1 sinon, 2 si les arguments sont erronés). Avec `--mark N`, la marque numéro N (à partir de 1) fixe l'échelle, et avec `--align` le dessin est aligné sur les marques, si bien que la capture montre les lignes de l'alignement : de l'endroit où le dessin place chaque marque jusqu'à la marque trouvée, avec la distance en millimètres (`show_residuals` dans `vision.cfg` les désactive). `examples/design_pair.csv` est un dessin pour les deux marques de `examples/marks.pgm`.

## 🧪 Tests

La suite compte 300 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et `eh3d.project.json` ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilent le programme, `test.bat` / `test.sh` lancent les tests et `run.bat` / `run.sh` l'exécutent avec un exemple. Les versions sont publiées sur GitHub.

### Téléchargements

Chaque version contient le programme avec sa fenêtre pour Windows (64 et 32 bits), Linux (64 bits) et macOS (Apple silicon et Intel), sous la forme `<nom>-<version>-<système>.zip` (Windows) ou `.tar.gz` avec le programme, la licence, le lisez-moi, l'icône et les exemples. Les fichiers sont construits par GitHub Actions, listés dans `SHA256SUMS` et signés avec la clé d'EH3D. Pour vérifier un téléchargement, comparez son empreinte avec le fichier `.sha256` qui l'accompagne (`sha256sum -c`) et vérifiez sa signature avec le fichier `.sig` qui l'accompagne (`ssh-keygen -Y verify`).

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande
│   ├── gui_main.rs                              la fenêtre comme programme à part (fonctionnalité gui)
│   ├── cli_args.rs                              les arguments de la ligne de commande, le même fichier dans chaque petit utilitaire
│   ├── align.rs                                 la rotation, le décalage et la taille d'une carte et son appariement avec le dessin
│   ├── batch.rs                                 un dossier d'images d'un coup, avec un rapport CSV
│   ├── config.rs                                le fichier de configuration : chaque valeur réglée
│   ├── cross.rs                                 repères fiduciaux en forme de croix
│   ├── fiducial.rs                              les repères ronds d'une image : seuil, taches et contrôles
│   ├── flatten.rs                               la lumière inégale retirée d'une image
│   ├── grid.rs                                  la distorsion de l'objectif à partir d'une grille de repères
│   ├── inspect.rs                               chaque pièce comparée au corps de son boîtier depuis une table, avec un verdict
│   ├── job.rs                                   le rapport d'un dossier lu dans un fil, avec progression et annulation
│   ├── lens.rs                                  les deux facteurs de l'objectif depuis une grille de marques, refusés s'ils ne collent pas
│   ├── points.rs                                les fichiers de points d'un dessin
│   ├── quality.rs                               si une image suffit pour mesurer : sa netteté, son contraste et son niveau moyen
│   ├── report.rs                                le rapport d'un dossier en une page HTML autonome, en huit langues
│   ├── ring.rs                                  les repères en anneau : ronds et percés, d'une ou deux tailles, avec confiance
│   ├── limit.rs                                 lecture d'un fichier avec limite de taille et un code pour chaque refus
│   ├── measure.rs                               pièces mesurées en millimètres, fichiers CSV et JSON
│   ├── offset.rs                                comment une pièce prise est tenue, avec la correction, la table d'un plateau et l'axe de la buse
│   ├── panels.rs                                les panneaux de la fenêtre : objectif, boîtiers, plan des images, décalages et contrôle de l'image
│   ├── scale.rs                                 des pixels aux millimètres et la distorsion radiale de l'objectif
│   ├── square.rs                                marques carrées et croix tournées d'un angle quelconque
│   ├── tools.rs                                 les outils du panneau latéral sur l'image : formes, pièces, rapport, échelle par le pas
│   ├── shot.rs                                  le mode capture de la fenêtre (--shot) : ses arguments, lus et vérifiés sans écran
│   ├── shots.rs                                 les positions de la caméra qui couvrent une pièce plus grande que son champ, par les tailles
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : image, repères, échelle, objectif et alignement
├── tests/
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   ├── strict.rs                                tests de la ligne de commande : options et mots qu'elle ne connaît pas
│   ├── read_limit.rs                            tests de la limite de taille des fichiers lus par la ligne de commande
│   ├── exchange.rs                              tests des commandes qui lisent une image et écrivent un fichier
│   ├── shot_sample.rs                           tests que les exemples s'ouvrent dans la fenêtre d'une capture
│   └── vision.rs                                tests de la bibliothèque et de la ligne de commande
├── examples/
│   ├── design.csv                               un exemple pour essayer le programme
│   ├── design_pair.csv                          un dessin pour les deux marques de marks.pgm
│   ├── marks.pgm                                un exemple pour essayer le programme
│   ├── measured.csv                             un exemple pour essayer le programme
│   ├── packages.csv                             une table de corps de boîtiers pour inspect
│   └── vision.cfg                               le fichier de configuration avec chaque valeur et sa signification
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   └── structure.json                           données de l'image de Structure
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── rust-toolchain.toml                          la toolchain Rust épinglée
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* La fenêtre n'a pas encore été vue sur un écran ; les marques sont appariées au dessin selon leur position, donc une carte très tournée ou une marque manquante est refusée au lieu d'être devinée.
* Il n'a été essayé ni sur une machine ni avec une collection de fichiers réels, et n'a pas eu de revue indépendante.
* Il lit un fichier image, pas une caméra en direct, ne trouve que des marques rondes, carrées et en forme de croix et a été essayé sur des images dessinées par les tests, pas sur des photos de vraies cartes. L'angle d'une pièce est mesuré à partir de ses pixels comme celui d'un rectangle, si bien qu'une pièce qui n'est pas un rectangle reçoit l'angle de son axe long.

**Ensuite**

* Lire la caméra en direct de la machine (EH3D-NODE-BUNGARD-AUX), essayer des photographies de vraies cartes et ajouter à la fenêtre l'entraînement de nouvelles marques.
* Voir la fenêtre sur un écran et la terminer.
* L'essayer avec des fichiers réels et avec les autres programmes de la suite.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON) : les crates partagés, dont le lecteur d'images
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): l'assemblage pick and place
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): le fraisage d'isolation du cuivre
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): le programme de commande de machine
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): le nœud de la tête de perçage avec la caméra
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): le centre qui trouve et lance les programmes de la suite

## 📚 Documentation

* [Journal des modifications](CHANGELOG.md)
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
