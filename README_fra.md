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
  <img src="https://img.shields.io/badge/Version-0.1.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`eh3d-vision`) avec 826 tests exécutés sur un ordinateur : trouve des marques fiduciaires rondes dans une image, convertit les pixels en millimètres et calcule la rotation et le décalage qui placent une carte où son dessin le dit. Il a une fenêtre (compilée avec `--features gui`, le programme `eh3d-vision-gui`) qui ouvre une image, montre les marques avec leurs coordonnées, fixe l'échelle, ajuste la distorsion de l'objectif avec une grille, aligne sur le dessin et exporte les options d'ISOCAM, mais la fenêtre n'a pas encore été vue sur un écran ; il n'a pas été essayé sur une machine et n'a pas été vérifié sur une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.1.3` |
| Maturité | Fonctionnel |
| Tests | 826 |
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
| Tests | 🟡 | 826 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées, et une caméra simulée au-dessus d'une carte qui ajoute du bruit, une mauvaise exposition, une image morte ou absente, une poussière qui ressemble à un repère ou un fichier d'image coupé, et qu'on interroge de nouveau. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`; une petite frontière unsafe seulement dans les moteurs de caméra compilés avec leur fonctionnalité) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🟠 | Écrite (egui, `--features gui`, le programme `eh3d-vision-gui`) : ouvrir ou déposer une image, voir les marques avec leurs coordonnées, fixer l'échelle avec une marque de taille connue, ajuster la distorsion de l'objectif avec une grille, aligner sur un dessin CSV et exporter les options d'ISOCAM ; sa logique est testée, mais elle n'a pas encore été compilée ni vue sur un écran. |
| Caméra en direct | 🟠 | la couche caméra, la mire d'étalonnage et l'alignement d'une carte sont écrits et testés sur une caméra et une table simulées ; les moteurs de Windows, V4L2 et libcamera sont des squelettes et rien n'a tourné sur une vraie caméra, optique ou carte. |
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

## 🖼️ Captures d'écran

<p align="center">
  <img src="images/screens/vision-studio-01-window.png" alt="La fenêtre avec l'image et le dessin d'exemple" width="100%">
</p>

<p align="center"><em>La fenêtre avec l'image et le dessin d'exemple : les deux repères trouvés, chacun avec son centre et son numéro, et les réglages de la recherche à gauche.</em></p>

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `eh3d-vision` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Ligne de commande stricte :** une option mal écrite (`--powr`) ou un mot en trop est une erreur qui la nomme, et non quelque chose d'ignoré ; sinon le programme ferait sa sortie avec les valeurs par défaut.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Limite de taille des fichiers lus :** la ligne de commande ne lit une image ou un fichier de points que s'il n'est pas plus grand que `max_file_mib` de `vision.cfg` (64 Mio par défaut) ; un fichier plus grand est refusé avec le code `file_too_large` et son nom, un fichier illisible avec `file_unreadable` et un fichier de points qui n'est pas du texte avec `file_not_text`, et rien n'est chargé.
* **Pas de code unsafe dans la compilation par défaut :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme ; seuls les moteurs de caméras en direct, compilés avec leur propre fonctionnalité, ont une petite frontière unsafe (quelques appels, dont les tailles de structures sont vérifiées à la compilation).
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
* **Marques par corrélation :** `eh3d-vision correlate IMAGE --diameter PX` fait glisser un disque de ce diamètre sur l'image et calcule à chaque endroit la corrélation normalisée (elle ne change pas si la lumière est plus forte ou le contraste plus faible) ; une marque que le seuil perd avec une lumière inégale est donc trouvée. Les endroits sont donnés à une fraction de pixel avec le score, le meilleur d'abord, aucun plus près d'un meilleur qu'un diamètre. Elle exige le diamètre (`correlate_diameter`, `correlate_min_score`) et est plus lente que la recherche par seuil : une image qui prendrait trop de temps est refusée (`correlate_too_big`).
* **Perspective corrigée :** `eh3d-vision rectify IMAGE --corners X1,Y1,...,X4,Y4 --output FICHIER.pgm` redessine l'image comme elle serait vue d'en haut, à partir des quatre coins d'un rectangle qui s'y trouve (une homographie avec échantillonnage bilinéaire) ; des coins qui ne forment pas un quadrilatère convexe sont refusés (`bad_corners`) et le plus grand côté est `rectify_max_side`.
* **Une rangée de pastilles :** `eh3d-vision pads IMAGE --mm-per-px V` prend les pièces de l'image pour des pastilles et donne le pas (la distance médiane le long de la droite d'ajustement), sa variation, l'écart d'une pastille à la rangée et leur taille moyenne, en CSV ou JSON ; le verdict est `fail` avec les raisons `pitch`, `row` et `expected` (`--expect-pitch`), selon `pads_tolerance`, et le code de sortie est 1.
* **Avant et après :** `eh3d-vision diff AVANT APRÈS` compare deux images de même taille : les pixels dont le niveau de gris diffère de plus de `diff_threshold`, leur part, la différence moyenne et la plus grande, le cadre qui les entoure et les régions d'au moins `diff_min_area` pixels ; `--mask FICHIER.pgm` écrit l'image des changements et le code de sortie est 1 quand plus de `diff_max_changed` a changé.
* **Dans quel sens une pièce est tenue :** `eh3d-vision orientation IMAGE --mm-per-px V` trouve la pièce (ce qui diffère de la carte) et la marque qu'elle porte (la bande d'une diode, le point de la broche 1 : ce qui diffère du corps à l'intérieur, même si cela ressemble à la carte) et donne la direction du milieu de la pièce vers la marque (degrés dans le sens antihoraire depuis l'axe x, y vers le haut), le quart de tour le plus proche et l'écart à celui-ci et, avec `--expect-deg`, l'écart à la direction voulue. La pièce est refusée (code de sortie 1, après l'écriture du fichier) comme `no_mark`, `centred`, `mark_big`, `wrong_way` ou, sans direction voulue, `skewed`.
* **L'empreinte des pastilles :** `eh3d-vision footprint IMAGE --pad-length MM --pad-width MM` mesure chaque pastille et la juge seule contre le dessin avec une tolérance en millimètres (`--tolerance`, `footprint_tolerance_mm`) : une ligne par pastille, de la rangée du haut vers le bas et de gauche à droite, avec les écarts et les raisons `length` et `width` ; `--count` vérifie aussi le nombre de pastilles (`count`), et le code de sortie est 1 quand une pastille est hors tolérance.
* **Une carte de la netteté de l'image :** `eh3d-vision sharpmap IMAGE --cells N` découpe l'image en grille et donne la netteté de chaque cellule (la variance du laplacien), la médiane, la cellule la plus faible, l'inclinaison de la netteté selon x et selon y et les cellules faibles (sous `--min-share` de la médiane) ; une cellule presque sans contraste est `empty` et n'est pas jugée. `--heat FICHIER.pgm` écrit l'image de la carte, sombre là où l'image est floue, et le code de sortie est 1 quand une cellule est faible (`uneven`).
* **L'échelle depuis une planche de carrés, avec un rapport :** `eh3d-vision board IMAGE --pitch-mm D --square-mm S` trouve les carrés, calcule la rotation de la planche et la distance entre voisins selon chaque axe, et donne les millimètres d'un pixel selon x, selon y et d'après le côté, avec le caractère carré des pixels (`aspect`) et l'écart des distances (`spread`) ; `--report FICHIER.txt` écrit le compte rendu pour une personne avec la ligne `mm_per_px = ...` pour `vision.cfg`. Elle est refusée (code de sortie 1) quand les pixels ne sont pas carrés (`aspect`), que les distances diffèrent (`spread`) ou que le côté et le pas ne concordent pas (`disagree`).
* **Quatre panneaux de plus dans la fenêtre :** le panneau latéral a la recherche par corrélation (les marques trouvées sont listées avec leur place et leur score), l'image redressée d'après les quatre marques trouvées (enregistrée comme `<image>-rectified.pgm`), la rangée de pastilles jugée d'après les pièces mesurées (enregistrée comme `<image>-pads.csv`) et la comparaison de l'image ouverte avec une autre (enregistrée comme `<image>-diff.csv` et `<image>-changes.pgm`) ; leurs valeurs sont des champs des panneaux (`correlate_diameter`, `correlate_min_score`, `rectify_max_side`, `pads_tolerance`, `diff_threshold`, `diff_min_area`, `diff_max_changed`) et leurs textes sont en huit langues. Leur logique est testée ; la fenêtre n'a pas été regardée sur un écran.
* **Le serveur de la machine :** `eh3d-vision server health|config|discover|check|detect|send` utilise les mêmes clés que les autres programmes de la suite (`server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns`) par le client partagé de la suite (`eh3d-serverclient` d'EH3D-COMMON) : `config` lit les valeurs de `vision.cfg` que le serveur garde pour la machine (l'artefact le plus récent `vision-<machine>.cfg` du format `text/x-eh3d-vision-config`, écrit comme les lignes du fichier ou comme un objet JSON ; tout ou rien ; les clés de la liaison ne sont jamais prises du serveur) et les écrit comme lignes pour le fichier, et `send quality|offset|pads|diff|orientation|footprint|sharpmap|board` garde sur le serveur le résultat JSON de cette commande comme l'artefact `vision-<kind>.json` (`POST /api/v1/artifacts`, avec la machine et le genre dans ses paramètres). Les nouvelles tentatives et l'état de la liaison sont ceux du client partagé (une requête qui a pu arriver n'est jamais envoyée deux fois), les refus du serveur peuvent être dits par leur code en huit langues, le jeton n'est jamais affiché, et on avertit d'un jeton qui irait en HTTP simple vers une adresse non locale. Le serveur a un greffon `vision` : les résultats sont donc aussi gardés selon sa convention (les puces suivantes) ; on teste contre un serveur en mémoire (`server::SimServer`) et un socket sur la boucle locale, pas contre le vrai serveur.
* **Résultats selon la convention du serveur :** `eh3d-vision server send fiducials|focus|blobs IMAGE` garde un résultat sous la forme que partagent le greffon `vision` d'EH3D-SERVER et les programmes qui regardent par une caméra : un fichier `vision-<type>.json` de format `application/json` et de schéma `eh3d.vision.result/1`, avec la taille et le SHA-256 de l'image, les constats du type (`fiducials` `{x, y, radius_px, score}`, `focus` `{score, sharp}`, `blobs` `{x, y, area_px}` ; `barcode` est vérifié mais jamais produit, car rien ici ne lit de codes), les unités (`px`) et la preuve (`picture`, sauf si `--evidence camera|simulator` dit autre chose : le programme ne peut pas savoir qu'un fichier vient d'une caméra). `server check FICHIER.json` dit si un document suit la convention et nomme la partie qui ne la suit pas, et un document qui ne la suit pas est refusé avant l'envoi. Le score d'une vérification de mise au point est 50 quand la netteté vaut `focus_reference`.
* **La station simulée du serveur :** `eh3d-vision server detect fiducials|barcode|focus|blobs IMAGE [--seed N] [--artifact]` demande au greffon `vision` du serveur (`POST /api/v1/work/jobs`, démarré et lu jusqu'à sa fin) le résultat qu'il invente à partir de la taille et du hachage de l'image et de la graine, toujours le même (`evidence: simulator`), dans la vue du travail ou gardé comme artefact. `station::simulate` est un portage du greffon avec lequel répond le serveur en mémoire, si bien que toute la chaîne travail, résultat et artefact est testée ici sans réseau ; elle n'a pas été exécutée contre le vrai serveur.
* **La recherche du serveur sur le réseau, comme vrai interrupteur :** `eh3d-vision server discover` liste le `hôte:port` des serveurs qui répondent à `_eh3d._tcp` sur le réseau local. Elle ne fait rien tant que `server_mdns` est éteint (par défaut), et le programme doit être compilé avec `--features mdns` (ceux des versions le sont) pour avoir le résolveur du client partagé. Ce qu'elle trouve, ce sont des candidats que la personne choisit, jamais des serveurs auxquels on se connecte seul, car n'importe quel appareil du réseau peut répondre.
* **Des marques entraînées avec un exemple, une carte très tournée et une liste sûre de serveurs :** `eh3d-vision train IMAGE --rect X,Y,W,H --output FICHIER` garde la moyenne de rectangles découpés autour d'une marque comme le fichier d'une marque (jamais écrit sur un fichier qui existe sauf avec `--replace`), et `eh3d-vision locate --mark FICHIER IMAGE` la retrouve par corrélation, tournée pas à pas si on le demande, avec sa position, son angle et son score (les clés `train_*` de `vision.cfg`) ; `eh3d-vision pair DESSIN.csv MESURES.csv --any-turn` apparie les marques au dessin quelle que soit la rotation de la carte, avec une marque manquante ou en trop, et refuse ce que deux façons expliquent aussi bien (les clés `pair_*`) ; `discovery` transforme ce qui répond sur le réseau en quelques candidats locaux bien formés (les clés `find_server_*`), et `server discover` et la fenêtre ne listent que ceux-là. Les commandes répondent dans la langue de `vision.cfg` ou de `--lang`, donnent du JSON avec `--json` et finissent avec le code de sortie 0 (fait), 1 (n'a pas pu être exécutée) ou 2 (exécutée, mais la vérification n'a pas été réussie : exemples qui ne concordent pas, marque indiscernable de ses voisines, marque introuvable, marques non appariables). Tout est essayé sur des images dessinées avec la vérité qui les a dessinées ; la boucle regarder, déplacer, regarder encore apparie toujours par position et la fenêtre n'entraîne pas encore de marques.
* **Un lecteur de trames JPEG animé, une carte de points à imprimer, la lumière d'une image et les trous d'une bande :** `eh3d-vision jpeg TRAME` lit une trame JPEG de base jusqu'à sa luminosité dans les limites de `mjpeg_max_bytes` et `mjpeg_max_pixels`, et une trame coupée, avec un marqueur là où il ne peut y en avoir, une table impossible ou une taille de milliards de pixels est refusée avec un code qui ne change pas (`jpeg_truncated`, `jpeg_bad_marker`, `jpeg_bad_table`, `jpeg_bad_size`, `jpeg_unsupported`, `jpeg_bad_data`), jamais par une panique ; `eh3d-vision calib-card` dessine une carte de points ronds en millimètres à imprimer à 100 %, la même à chaque fois et refusée si elle ne tient pas sur sa page (les clés `card_*`) ; `eh3d-vision light-check IMAGE` dit si la lumière d'une image est bonne (moyenne, part brûlée, zones, inclinaison ; les clés `light_*`) et pourquoi non, avec le code de sortie 2 si elle ne l'est pas ; `eh3d-vision tape-locate IMAGE` trouve les trous d'une bande de pièces, le pas par rapport au pas de la bande et les alvéoles (les clés `tape_*`). Tout est essayé sur des images et des trames dessinées avec une graine fixe : aucune caméra n'a servi, aucune carte n'a été imprimée et aucune bande n'a été photographiée.
* **Un bord à une fraction de pixel, avec son incertitude :** `eh3d-vision edge IMAGE --from X,Y --to X,Y` lit l'image le long d'une ligne tracée à travers un bord (bilinéaire, espacée de `edge_step_px`), prend les deux bouts de la ligne pour les deux fonds plats, trouve où le profil croise le niveau à mi-chemin et donne la position le long de la ligne et dans l'image, le contraste, la largeur de la transition et l'incertitude (un écart type : le bruit des fonds, l'arrondi de 8 bits ajouté, sur la pente à l'endroit du croisement). Le code de sortie est 1 quand l'incertitude dépasse `edge_max_uncertainty_px` ; une ligne sans bord est `edge_flat`. Un bord plus net qu'un pixel peut pencher jusqu'à 0,05 px.
* **L'histogramme et le seuil automatique :** `eh3d-vision histogram IMAGE --bins N` donne les niveaux de gris en 8 à 256 groupes et les coupures d'Otsu (celle qu'utilise la recherche de marques), d'isodata et de la méthode du triangle, la part de variance qu'expliquent les deux groupes d'Otsu, la part de pixels noirs et blancs et si les trois coupures concordent (à 8 niveaux près) ; sinon, il faut regarder l'éclairage ou le `threshold` de `vision.cfg`.
* **Une image contre une référence, par un masque de tolérance :** `eh3d-vision refcheck RÉFÉRENCE IMAGE --tolerance MASQUE.pgm` compare deux images de même taille où l'écart permis peut changer d'un endroit à l'autre : un pixel du masque à 0 garde le `diff_threshold` général, de 1 à 254 permet autant de niveaux de gris en plus, et 255 n'est pas regardé. Il donne les pixels hors tolérance, de combien ils la dépassent, en combien de régions (`diff_min_area`) et où ; `--map FICHIER.pgm` les écrit comme une image et le code de sortie est 1 au-delà de `diff_max_changed`.
* **Les résidus de l'objectif, marque par marque :** `eh3d-vision lens IMAGE --residuals` écrit, pour chaque marque de la grille, de combien elle s'écarte de sa place avant et après la correction, la marque la plus éloignée, les trois bandes de distance au centre et le rapport de la bande extérieure à l'intérieure : un rapport bien supérieur à 1 dit que deux termes radiaux ne suffisent pas pour l'objectif (ou que la mire n'est pas plane).
* **Plus dans la fenêtre :** le panneau latéral a les réglages du serveur (le panneau de la suite d'`eh3d-view` : adresse, jeton, machine, nouvelles tentatives, l'interrupteur de la recherche sur le réseau, connexion et sessions), l'état de la liaison dans le coin, la recherche de serveurs (seulement interrupteur allumé, dans un fil, avec des candidats à choisir), un histogramme avec les trois coupures, le bord le long d'une ligne de quatre nombres, la vérification contre une référence avec masque de tolérance, et une ligne d'aide repliée sous les titres des panneaux, dans les huit langues. La fenêtre n'a pas été regardée sur un écran.
* **Tests à graine fixe :** `tests/fuzz.rs` fournit des images de toute taille, des coins n'importe où, des pastilles de toute taille, des carrés n'importe où, des fichiers de configuration, du JSON, du Base64 et des réponses HTTP abîmés, depuis un générateur qui ne lit pas l'horloge ; rien ne doit paniquer et ce qui sort doit avoir du sens, et les panneaux de la fenêtre traversent de même des images aléatoires. `tests/fuzz_camera.rs` fait de même pour un bord le long de n'importe quelle ligne de n'importe quelle image, l'histogramme, une image contre une référence par n'importe quel masque, les résidus de marques quelconques et des documents de la convention coupés, altérés ou remplis de déchets.
* **Une caméra en direct derrière un trait, essayée seulement avec une caméra simulée (`src/capture/`) :** les caméras sont listées par nom avec une clé qui ne change pas (`vendor:product:serial@port`), ouvertes dans un format (YUYV, JPEG animé, RVB, gris) et donnent des images avec un compteur et leurs octets d'origine ; chacune des 13 propriétés a sa plage ou la caméra dit qu'elle ne l'a pas. La charge d'un bus USB (`capture_usb_mbps`), les images noires, répétées, perdues ou coupées et le scintillement de la lumière sont repérés, on attend de nouvelles images par leur nombre et non par des secondes, et le journal va vers une fonction. La caméra simulée tombe en panne exprès ; les moteurs de Windows, V4L2 et libcamera sont des squelettes derrière les fonctionnalités `mfoundation`, `v4l2` et `libcamera` (désactivées par défaut) et n'ont pas été essayés sur une vraie caméra, et les images JPEG animé sont vérifiées mais pas encore décodées.
* **Une mire d'étalonnage à marqueurs numérotés (`src/charuco/`) :** `eh3d-vision charuco card` dessine un damier dont les cases blanches portent des marqueurs, en millimètres pour imprimer à 100 % ; `eh3d-vision charuco calibrate IMAGE` lit les marqueurs, trouve les coins à une fraction de pixel quel que soit l'angle et ajuste la correspondance pixels-millimètres de trois façons (homographie, affine, similitude). Elle est refusée si elle laisse trop d'écart aux coins (`calib_max_residual_px`), si les méthodes placent les coins trop loin les uns des autres (`calib_max_method_gap`) ou si elle ne correspond pas à la distance mesurée à la règle entre les deux coins extérieurs d'une rangée (`calib_check_mm`) ; sans règle elle n'est pas utilisée sauf si `calib_allow_unchecked` le permet. Le seul décalage est celui de `camera_offset_x_mm`, `camera_offset_y_mm` et `camera_offset_deg`, et un étalonnage fait d'images simulées est refusé pour une vraie caméra.
* **Une carte remise en place en regardant, déplaçant et regardant de nouveau (`src/boardaim.rs`) :** les repères sont regardés depuis une pose qui ne change pas, la rotation et le décalage sont calculés et la table déplace la carte du contraire (jamais plus de `align_max_step_mm` et `align_max_step_deg` à la fois, ni `align_max_total_mm` au total) ; après avoir attendu des images prises après le déplacement, on regarde de nouveau, jusqu'à ce que la carte soit dans `align_tolerance_mm` et `align_tolerance_deg`. Quand quelque chose ne va pas, on ne va que vers l'état le plus sûr (pas entiers, demi-pas, arrêt sur place, abandon) : une erreur qui grandit divise le pas par deux puis arrête, et l'absence de repères, des images noires ou répétées, une limite ou une panne de la table arrêtent les déplacements. Elle n'a tourné qu'avec une table simulée.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Un rapport ou un fichier de mesures qui écraserait un autre fichier | on n'écrit que là où il n'y en a pas ; il faut `--replace` pour écraser, et on n'écrit jamais à travers un lien |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)`; une petite frontière unsafe seulement dans les moteurs de caméra compilés avec leur fonctionnalité |
| Un jeton affiché ou envoyé là où il ne le faut pas | le jeton n'est jamais affiché ni mis dans un résultat ; les valeurs qu'envoie un serveur sont prises toutes ou aucune et les clés de la liaison (`server_*`) ne sont jamais prises de lui ; on avertit d'un jeton qui irait en HTTP simple vers une adresse non locale |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_vision_studio` (`use eh3d_vision_studio::...`) ; la ligne de commande est `eh3d-vision`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Seule la commande `server` utilise le réseau (HTTP simple vers le serveur de la machine indiqué dans `vision.cfg`) ; toutes les autres travaillent sur des fichiers locaux.

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

Les commandes `train`, `locate`, `pair`, `light-check` et `tape-locate` finissent avec le code de sortie 0 quand elles sont faites, 1 quand elles n'ont pas pu être exécutées (une option erronée, un fichier illisible ou impossible à écrire, une limite dépassée) et 2 quand elles ont été exécutées mais que leur vérification n'a pas été réussie.

## ⚙️ Configuration

Les valeurs réglées se trouvent dans un simple fichier `clé = valeur`, pas dans le code : `vision.cfg` dans le dossier où le programme démarre ou à côté de lui (la variable `EH3D_VISION_CONFIG` désigne un autre fichier). Il fixe la langue de la fenêtre, si les marques sont sombres ou claires, le seuil, la plage du diamètre, le remplissage et l'allongement d'une marque, la taille de la marque qui fixe l'échelle, les colonnes et les lignes de la grille, si l'alignement peut changer la taille, les limites qu'un alignement doit respecter et le plus gros fichier lu. `examples/vision.cfg` énumère chaque clé avec sa signification et la valeur avec laquelle le programme démarre ; sans fichier, ces valeurs s'appliquent, et un fichier avec une erreur est refusé avec sa ligne. En ligne de commande les mêmes choses se donnent comme options (voir Interfaces). Il contient aussi les autres valeurs réglées : la confiance (`refine_subpixel`, `full_contrast`), la fenêtre de la lumière (`flatten_window`), l'échelle et la taille d'une pièce (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), l'étendue d'une croix (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), la tolérance d'une mire (`pitch_tolerance`) et le nombre maximal d'images d'un dossier (`batch_max_files`). Le remplissage minimal d'un carré et le score minimal d'une croix à n'importe quel angle sont `square_min_fill` et `cross_min_score`. La plage du côté d'un carré et de combien sa boîte peut être plus longue que large sont `square_min_size`, `square_max_size` et `square_max_elongation`, à part celles de la croix. L'objectif est `lens_k1` et `lens_k2` (tels que `lens` les calcule) et l'écart maximal d'une grille à son réseau après la correction est `lens_max_rms`. La table des boîtiers que lit `inspect` est `package_table`, et l'écart maximal d'une pièce à son boîtier et sa rotation maximale sont `inspect_tolerance` et `inspect_max_angle`. Le score sous lequel le rapport HTML avertit à propos d'une image est `report_low_score`. La distance entre les marques de la mire qui fixe l'échelle est `pitch_mm`, et `replace_output` dit si les fichiers écrits peuvent l'être sur un fichier existant (non par défaut). Le recouvrement des images voisines d'un plan de `shots` et la marge autour de la pièce sont `shots_overlap` et `shots_margin`. Que la fenêtre dessine ou non le réseau de l'objectif se règle avec `show_lens_grid`. Le boîtier face auquel `inspect` et la fenêtre jugent les pièces est `inspect_package`, et la taille de la pièce pour laquelle `shots` et la fenêtre planifient les images est `shots_part_width` et `shots_part_height` (0 et 0 : le corps du boîtier). Les repères en anneau ont la plage de leur diamètre extérieur (`ring_min_diameter`, `ring_max_diameter`), du rapport du diamètre intérieur à l'extérieur (`ring_min_ratio`, `ring_max_ratio`), le remplissage minimal de leur bande (`ring_min_fill`), l'allongement (`ring_max_elongation`), le diamètre qui sépare les deux tailles (`ring_split`, 0 pour le trouver) et l'écart entre les tailles (`ring_size_gap`). L'axe de la buse dans l'image d'une pièce prise est `nozzle_x_mm` et `nozzle_y_mm`, et la distance maximale où elle peut être tenue et la rotation maximale sont `pick_max_offset_mm` et `pick_max_turn_deg` (0 : non vérifié). La rotation d'une pièce bien tenue est `nominal_angle_deg`, le fait que la caméra voie les pièces en miroir est `pick_mirror`, et le contrôle de l'image est `quality_check` avec les minimums `quality_min_contrast`, `quality_min_mean`, `quality_max_mean` et `quality_min_sharpness` (0 : non vérifié). Le serveur est `server_address`, `server_token`, `server_machine`, `server_timeout_s` et `server_retries` (adresse vide : pas de serveur) ; la recherche par corrélation est `correlate_diameter` et `correlate_min_score` ; le plus grand côté d'une image redressée est `rectify_max_side` ; la tolérance d'une rangée de pastilles est `pads_tolerance` ; et la comparaison de deux images est `diff_threshold`, `diff_min_area` et `diff_max_changed`. La recherche du serveur sur le réseau local se règle avec `server_mdns` (éteinte par défaut). La lecture d'une pièce marquée est `orientation_body_contrast`, `orientation_mark_contrast`, `orientation_min_mark_px` et `orientation_tolerance_deg` ; la tolérance d'une pastille est `footprint_tolerance_mm` ; la carte de netteté est `sharpmap_cells`, `sharpmap_min_share` et `sharpmap_min_contrast` ; et l'échelle depuis une planche est `board_square_mm` (la distance entre centres est `pitch_mm`), `board_max_aspect` et `board_max_spread`. Un `vision.cfg` qui contient un jeton doit rester privé. La netteté qui compte pour 50 points dans un résultat de mise au point est `focus_reference`, et le pas le long de la ligne d'un bord et la plus grande incertitude de sa position sont `edge_step_px` et `edge_max_uncertainty_px`. La caméra en direct, la mire d'étalonnage et l'alignement d'une carte ont leurs propres clés (`capture_*`, `calib_*`, `camera_offset_*` et `align_*`) : chacune a une plage, une valeur hors plage est refusée et une erreur qu'une clé gouverne nomme la clé. Le lecteur de trames JPEG animé, la carte de points, la vérification de la lumière et la recherche des trous d'une bande ont leurs propres clés (`mjpeg_*`, `card_*`, `light_*` et `tape_*`) : chacune a une plage, et une valeur hors de celle-ci est refusée en nommant la clé.

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

La suite compte 826 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

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
│   ├── board.rs                                 l'échelle depuis une planche de carrés : selon x, selon y et d'après le côté, avec rapport
│   ├── boardaim.rs                              une carte placée où son dessin le dit : regarder, déplacer, regarder, toujours vers le plus sûr
│   ├── capture/                                 la caméra en direct : trait, formats, propriétés, budget USB, santé du flux, caméra simulée
│   ├── charuco/                                 la mire d'étalonnage à marqueurs numérotés : coins, étalonnage, fichier et carte
│   ├── config.rs                                le fichier de configuration : chaque valeur réglée
│   ├── correlate.rs                             marques rondes trouvées par corrélation avec un disque du diamètre voulu
│   ├── diff.rs                                  deux images du même endroit comparées : ce qui a changé, où et de combien
│   ├── discovery.rs                             réponses du réseau en une courte liste de candidats sûrs, et un résolveur scénarisé
│   ├── cross.rs                                 repères fiduciaux en forme de croix
│   ├── edge.rs                                  un bord à une fraction de pixel le long d'une ligne, avec son incertitude
│   ├── fiducial.rs                              les repères ronds d'une image : seuil, taches et contrôles
│   ├── flatten.rs                               la lumière inégale retirée d'une image
│   ├── footprint.rs                             chaque pastille jugée contre la taille du dessin, avec une tolérance en millimètres
│   ├── grid.rs                                  la distorsion de l'objectif à partir d'une grille de repères
│   ├── inspect.rs                               chaque pièce comparée au corps de son boîtier depuis une table, avec un verdict
│   ├── job.rs                                   le rapport d'un dossier lu dans un fil, avec progression et annulation
│   ├── lens.rs                                  les deux facteurs de l'objectif depuis une grille de marques, refusés s'ils ne collent pas
│   ├── points.rs                                les fichiers de points d'un dessin
│   ├── quality.rs                               si une image suffit pour mesurer : sa netteté, son contraste et son niveau moyen
│   ├── reference.rs                             une image contre une référence par un masque de tolérance
│   ├── histogram.rs                             l'histogramme et les coupures d'Otsu, d'isodata et du triangle
│   ├── report.rs                                le rapport d'un dossier en une page HTML autonome, en huit langues
│   ├── ring.rs                                  les repères en anneau : ronds et percés, d'une ou deux tailles, avec confiance
│   ├── limit.rs                                 lecture d'un fichier avec limite de taille et un code pour chaque refus
│   ├── measure.rs                               pièces mesurées en millimètres, fichiers CSV et JSON
│   ├── offset.rs                                comment une pièce prise est tenue, avec la correction, la table d'un plateau et l'axe de la buse
│   ├── orientation.rs                           dans quel sens est tenue une pièce marquée, d'après la direction de la marque
│   ├── pads.rs                                  une rangée de pastilles : le pas, sa variation et l'écart d'une pastille à la rangée
│   ├── panels.rs                                les panneaux de la fenêtre : objectif, boîtiers, plan des images, décalages et contrôle de l'image
│   ├── perspective.rs                           perspective corrigée : l'image redessinée vue d'en haut, à partir de quatre coins
│   ├── scale.rs                                 des pixels aux millimètres et la distorsion radiale de l'objectif
│   ├── server.rs                                le chemin vers le serveur EH3D par le client partagé et un serveur en mémoire pour les tests
│   ├── convention.rs                            le résultat de vision selon la convention du serveur : le faire, le vérifier, SHA-256
│   ├── station.rs                               la station de vision simulée du serveur : le travail, sa réponse, un portage
│   ├── sharpmap.rs                              une carte de la netteté de l'image : une grille de cellules, les faibles et l'inclinaison
│   ├── square.rs                                marques carrées et croix tournées d'un angle quelconque
│   ├── tools.rs                                 les outils du panneau latéral sur l'image : formes, pièces, rapport, échelle par le pas
│   ├── train.rs                                 une marque entraînée sur des exemples, retrouvée par corrélation à son angle
│   ├── turnpair.rs                              marques appariées au dessin pour une carte tournée à tout angle
│   ├── dotcard.rs                               la carte de points ronds à imprimer à 100 %, en dessin en millimètres
│   ├── light.rs                                 si la lumière d'une image est bonne : moyenne, brûlé, zones et inclinaison
│   ├── mjpeg.rs                                 un lecteur de trames JPEG animé qui ne se fie jamais à ses octets
│   ├── tape.rs                                  les trous et alvéoles d'une bande de pièces et le pas par rapport à celui de la bande
│   ├── verbs.rs                                 les commandes train, locate, pair, light-check, tape-locate, calib-card et jpeg
│   ├── shot.rs                                  le mode capture de la fenêtre (--shot) : ses arguments, lus et vérifiés sans écran
│   ├── shots.rs                                 les positions de la caméra qui couvrent une pièce plus grande que son champ, par les tailles
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── checks.rs                                panneaux de la fenêtre : corrélation, redressement, rangée de pastilles, comparaison
│   ├── readings.rs                              panneaux de la fenêtre : histogramme, bord, référence et réglages du serveur
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : image, repères, échelle, objectif et alignement
├── tests/
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   ├── strict.rs                                tests de la ligne de commande : options et mots qu'elle ne connaît pas
│   ├── read_limit.rs                            tests de la limite de taille des fichiers lus par la ligne de commande
│   ├── exchange.rs                              tests des commandes qui lisent une image et écrivent un fichier
│   ├── commands.rs                              tests des commandes correlate, rectify, pads, diff et server
│   ├── analysis.rs                              tests des commandes orientation, footprint, sharpmap et board
│   ├── capture.rs                               caméra en direct, étalonnage et alignement sur la caméra simulée, et la commande
│   ├── fuzz.rs                                  tests avec entrées aléatoires et graine fixe : rien ne doit paniquer
│   ├── camera.rs                                les nouvelles commandes lancées comme programme : edge, histogram, refcheck, résidus, server
│   ├── fuzz_camera.rs                           entrées aléatoires à graine fixe pour les nouvelles mesures et la convention
│   ├── fuzz_capture.rs                          entrées aléatoires à graine fixe pour lecteurs d'image, étalonnage, alignement
│   ├── training.rs                              une marque entraînée retrouvée au lieu et à l'angle qui l'ont dessinée, avec ses fichiers
│   ├── pairing.rs                               marques d'une carte tournée à tout angle appariées selon la vérité
│   ├── mjpeg.rs                                 trames codées relues, coupées partout, octets changés, tailles énormes, bruit
│   ├── newcli.rs                                les nouvelles commandes : texte, JSON, fichiers respectés, codes de sortie, exemples
│   ├── lightfuzz.rs                             lumière, bande et carte avec n'importe quelle image et réglage, graine fixe
│   ├── discovery.rs                             les réponses du réseau, hostiles ou nombreuses, ne donnent jamais un candidat risqué
│   ├── shot_sample.rs                           tests que les exemples s'ouvrent dans la fenêtre d'une capture
│   ├── vision.rs                                tests de la bibliothèque et de la ligne de commande
│   └── reqs.rs                                  tests du registre des exigences
├── examples/
│   ├── design.csv                               un exemple pour essayer le programme
│   ├── design_pair.csv                          un dessin pour les deux marques de marks.pgm
│   ├── marks.pgm                                un exemple pour essayer le programme
│   ├── measured.csv                             un exemple pour essayer le programme
│   ├── packages.csv                             une table de corps de boîtiers pour inspect
│   └── vision.cfg                               le fichier de configuration avec chaque valeur et sa signification
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   ├── NOT_YET.md                               de bonnes idées hors de cette version, avec pourquoi et quand
│   ├── REQUIREMENTS.md                          le registre des exigences : état, preuve et test de chacune
│   ├── SCOPE.md                                 à quoi il sert, ce qu'il ne fera pas et ce qu'il ne décide jamais
│   ├── STATUS.md                                ce qui existe, ce qui manque et ce qui reste ouvert
│   └── structure.json                           données de l'image de Structure
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── rust-toolchain.toml                          la toolchain Rust épinglée
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── AGENTS.md                                    les règles pour qui modifie le dépôt
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* La fenêtre n'a pas encore été vue sur un écran ; `align` et la boucle regarder, déplacer, regarder encore apparient les marques au dessin selon leur position, donc une carte très tournée ou une marque manquante y est refusée (`pair --any-turn` les apparie).
* Il n'a été essayé ni sur une machine ni avec une collection de fichiers réels, et n'a pas eu de revue indépendante.
* Il lit des fichiers image, et une caméra en direct seulement en simulation (les moteurs de Windows, V4L2 et libcamera sont des squelettes jamais essayés sur une vraie caméra), ne trouve que des marques rondes, carrées et en forme de croix et a été essayé sur des images dessinées par les tests, pas sur des photos de vraies cartes. L'angle d'une pièce est mesuré à partir de ses pixels comme celui d'un rectangle, si bien qu'une pièce qui n'est pas un rectangle reçoit l'angle de son axe long.
* Le serveur n'a pas de greffon de vision : le programme utilise donc les artefacts du magasin (`POST /api/v1/artifacts` pour un résultat, `GET /api/v1/artifacts` et `GET /api/v1/artifacts/{id}` pour la configuration d'une machine) avec des noms et un format propres (`vision-<kind>.json`, `vision-<machine>.cfg`) ; aujourd'hui on ne teste que contre un serveur en mémoire et un socket sur la boucle locale, et la recherche du serveur par mDNS (`server_mdns`) est un interrupteur sans résolveur intégré.

**Ensuite**

* Lire la caméra en direct de la machine (EH3D-NODE-BUNGARD-AUX), essayer des photographies de vraies cartes et mettre dans la fenêtre l'entraînement de nouvelles marques, que les commandes ont déjà.
* Voir la fenêtre sur un écran et la terminer.
* L'essayer avec des fichiers réels et avec les autres programmes de la suite.
* Écrire les moteurs en direct (images de Media Foundation, V4L2 et libcamera), essayer la mire d'étalonnage avec une vraie optique, la carte de points sur papier et l'alignement avec une vraie table.

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
* [Périmètre](docs/SCOPE.md): à quoi sert le programme, ce qu'il ne fera pas et ce qu'il ne décide jamais.
* [Exigences](docs/REQUIREMENTS.md): le registre des capacités, l'état de chacune et le test qui la montre.
* [État](docs/STATUS.md): ce qui existe, ce qui manque et ce qui reste ouvert.
* [Pas encore](docs/NOT_YET.md): de bonnes idées laissées de côté dans cette version.
* [Règles pour modifier le dépôt](AGENTS.md): les règles pour qui modifie le dépôt.
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
