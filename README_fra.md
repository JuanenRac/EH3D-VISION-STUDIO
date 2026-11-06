<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciaires dans une image de caméra : marques, échelle et correction d'une carte

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`eh3d-vision`) avec 138 tests exécutés sur un ordinateur : trouve des marques fiduciaires rondes dans une image, convertit les pixels en millimètres et calcule la rotation et le décalage qui placent une carte où son dessin le dit. Il a une fenêtre (compilée avec `--features gui`, le programme `eh3d-vision-gui`) qui ouvre une image, montre les marques avec leurs coordonnées, fixe l'échelle, ajuste la distorsion de l'objectif avec une grille, aligne sur le dessin et exporte les options d'ISOCAM, mais la fenêtre n'a pas encore été vue sur un écran ; il n'a pas été essayé sur une machine et n'a pas été vérifié sur une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.3` |
| Maturité | Échafaudage |
| Tests | 138 |
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
| Tests | 🟡 | 138 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées. |
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
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.
* **Les repères pour ISOCAM :** `align --isocam-args` écrit chaque paire de repères comme l'option `--fiducial X_CONCEPTION,Y_CONCEPTION,X_MESURE,Y_MESURE` de `eh3d-isocam`.
* **Une fenêtre :** compilée avec `--features gui` comme le programme `eh3d-vision-gui` (sous Windows elle s'ouvre sans console) : elle ouvre une image PNG ou PGM avec zoom et déplacement, trouve les marques avec des valeurs modifiables, les dessine avec leurs coordonnées, fixe l'échelle avec une marque de taille connue, ajuste la distorsion de l'objectif avec une grille de marques, aligne sur un dessin d'un fichier CSV (les marques lui sont appariées selon leur position, donc l'ordre n'importe pas) et exporte les options `--fiducial` d'ISOCAM seulement quand l'alignement est accepté. Elle parle les huit langues et n'a pas besoin de réseau.
* **Un fichier de configuration :** `vision.cfg` contient chaque valeur réglée (l'aspect d'une marque, la grille de la mire, les limites d'un alignement accepté, le plus gros fichier lu) ; une erreur dedans est refusée avec sa ligne, et `examples/vision.cfg` les énumère toutes.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_vision_studio` (`use eh3d_vision_studio::...`) ; la ligne de commande est `eh3d-vision`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configuration

Les valeurs réglées se trouvent dans un simple fichier `clé = valeur`, pas dans le code : `vision.cfg` dans le dossier où le programme démarre ou à côté de lui (la variable `EH3D_VISION_CONFIG` désigne un autre fichier). Il fixe la langue de la fenêtre, si les marques sont sombres ou claires, le seuil, la plage du diamètre, le remplissage et l'allongement d'une marque, la taille de la marque qui fixe l'échelle, les colonnes et les lignes de la grille, si l'alignement peut changer la taille, les limites qu'un alignement doit respecter et le plus gros fichier lu. `examples/vision.cfg` énumère chaque clé avec sa signification et la valeur avec laquelle le programme démarre ; sans fichier, ces valeurs s'appliquent, et un fichier avec une erreur est refusé avec sa ligne. En ligne de commande les mêmes choses se donnent comme options (voir Interfaces).

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.88 ou plus récente). Le dépôt EH3D-COMMON doit être cloné à côté de celui-ci : le lecteur d'images en est pris par chemin.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Pour prendre une capture de la fenêtre sans l'ouvrir à la main, lancez-la avec `--shot FICHIER.png` (aussi `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`) : elle s'ouvre à cette taille, enregistre le PNG, ne garde aucun réglage et se ferme toute seule (code de sortie 0 si le fichier a été écrit, 1 sinon, 2 si les arguments sont erronés).

## 🧪 Tests

La suite compte 138 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

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
│   ├── config.rs                                le fichier de configuration : chaque valeur réglée
│   ├── fiducial.rs                              les repères ronds d'une image : seuil, taches et contrôles
│   ├── grid.rs                                  la distorsion de l'objectif à partir d'une grille de repères
│   ├── points.rs                                les fichiers de points d'un dessin
│   ├── scale.rs                                 des pixels aux millimètres et la distorsion radiale de l'objectif
│   ├── shot.rs                                  le mode capture de la fenêtre (--shot) : ses arguments, lus et vérifiés sans écran
│   ├── browse.rs                                la liste des fichiers de la fenêtre d'ouverture d'un fichier
│   ├── gui.rs                                   la fenêtre (egui, avec la fonctionnalité gui) : dessine l'état et transmet ce que fait la personne
│   ├── lang.rs                                  les mots de la fenêtre dans les huit langues
│   ├── viewport.rs                              zoom et déplacement de l'image, en simples nombres
│   └── window.rs                                l'état de la fenêtre sans écran : image, repères, échelle, objectif et alignement
├── tests/
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   ├── strict.rs                                tests de la ligne de commande : options et mots qu'elle ne connaît pas
│   ├── shot_sample.rs                           tests que les exemples s'ouvrent dans la fenêtre d'une capture
│   └── vision.rs                                tests de la bibliothèque et de la ligne de commande
├── examples/
│   ├── design.csv                               un exemple pour essayer le programme
│   ├── marks.pgm                                un exemple pour essayer le programme
│   ├── measured.csv                             un exemple pour essayer le programme
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
* Il lit un fichier image, pas une caméra en direct, ne trouve que des marques rondes (ni croix ni carrés) et a été essayé sur des images dessinées par les tests, pas sur des photos de vraies cartes.

**Ensuite**

* Lire la caméra en direct de la machine (EH3D-NODE-BUNGARD-AUX), trouver des croix et des carrés, essayer des photographies de vraies cartes et ajouter à la fenêtre l'entraînement de nouvelles marques.
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
