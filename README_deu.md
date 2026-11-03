<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Passmarken in einem Kamerabild: Marken, Maßstab und Korrektur einer Platine

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute wirklich läuft:** eine Rust-Bibliothek und ein Kommandozeilenprogramm (`eh3d-vision`) mit 66 Tests, die auf einem Computer laufen: findet runde Passmarken in einem Bild, wandelt Pixel in Millimeter um und berechnet Drehung und Verschiebung, die eine Platine dorthin bringen, wo ihr Entwurf sie haben will. Es hat noch kein Fenster, wurde an keiner Maschine ausprobiert und nicht an einer Sammlung echter Dateien geprüft.

---

> **Sicherheitsstatus:** Dieses Programm steuert keine Maschine und ist kein Sicherheitssystem. Was es erzeugt, muss ein Mensch prüfen, bevor etwas an einer Maschine läuft; nichts davon lief auf echter Hardware.


## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.2` |
| Reifegrad | Gerüst |
| Tests | 66 |
| Technologie-Stack | Rust |
| Auf echter Hardware | Nicht erprobt: nichts davon lief an einer Maschine |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur](#-struktur-des-repositorys) · [Verwandte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Kern | 🟡 | findet runde Passmarken in einem Bild, wandelt Pixel in Millimeter um und berechnet Drehung und Verschiebung, die eine Platine dorthin bringen, wo ihr Entwurf sie haben will; am Computer mit normalen und beschädigten Eingaben getestet, nicht mit einer Sammlung echter Dateien. |
| Kommandozeile | 🟡 | `eh3d-vision` führt dieselben Funktionen wie die Bibliothek aus und gibt das Ergebnis aus; jeder Fehler kommt als Meldung mit einem Exit-Code ungleich null zurück. |
| Tests | 🟡 | 66 Tests bestehen (`cargo test`), mit Normalfällen, Grenzfällen und beschädigten Eingaben. |
| Sicherheit | 🟡 | Reines sicheres Rust (`forbid(unsafe_code)`): nichts gerät bei den übergebenen Dateien in Panik und ein schlechter Wert ist ein Fehler mit Begründung; keine unabhängige Prüfung. |
| Fenster | 🔴 | Geplant: das egui-Fenster, das die Suite gemeinsam nutzt; heute gibt es nur die Kommandozeile. |
| Echte Hardware | 🔴 | Nicht erprobt: nichts davon wurde an einer Maschine ausgeführt. |
| Dokumentation | 🟡 | README in acht Sprachen, das Changelog und die zwei aus `docs/` gezeichneten Bilder. |

## 🎯 Überblick

* **Die Marken** (`src/fiducial.rs`): Das Bild wird mit der Otsu-Schwelle in dunkel und hell geteilt, die zusammenhängenden Flecken werden vermessen, und die, die Scheiben der erwarteten Größe sind (Fläche, Füllung des umschließenden Kreises und Elongation), sind die Marken, die besten zuerst; ein vom Bildrand abgeschnittener Fleck bleibt draußen, weil sein Mittelpunkt nicht vertrauenswürdig ist, und die Marken können dunkel auf hell oder hell auf dunkel sein.
* **Maßstab und Objektiv** (`src/scale.rs`): Die Millimeter eines Pixels kommen von einer Marke bekannter Größe, die Maschinenachsen gehen nach oben, während das Bild nach unten geht, und die radiale Verzeichnung des Objektivs wird mit kleinsten Quadraten aus einem Raster von Marken gewonnen und korrigiert.
* **Die Platine** (`src/align.rs`): Aus zwei oder mehr Passmarken des Entwurfs und denselben, wie die Kamera sie fand, werden Drehung, Verschiebung und (wenn erlaubt) Größe mit kleinsten Quadraten berechnet, samt Restfehler in Millimetern; eine Transformation mit zu großem Rest (eine falsche Marke) oder abweichender Größe (eine gedehnte Platine) wird nicht akzeptiert.

## 🧩 Rolle in EH3D

EH3D-VISION-STUDIO ist ein Programm der EH3D Fabrication Suite. Heute arbeitet es als Bibliothek und Kommandozeile; sein Fenster im gemeinsamen Aussehen der Suite ist geplant. Es liest und schreibt einfache Dateien und lässt sich allein oder zwischen den anderen Programmen nutzen.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-VISION-STUDIO: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>


Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## ✨ Funktionen

* **Bibliothek und Kommandozeile:** Jeder Befehl von `eh3d-vision` ist eine Funktion der Bibliothek, sodass ein anderes Programm der Suite ihn ohne Prozessstart aufrufen kann.
* **Strenge Kommandozeile:** Eine falsch geschriebene Option (`--powr`) oder ein überzähliges Wort ist ein Fehler, der sie nennt, und wird nicht ignoriert; sonst würde das Programm seine Ausgabe mit den Standardwerten erzeugen.
* **Tests mit beschädigten Eingaben:** Die Tests füttern auch leere Dateien, Datenmüll, riesige Zahlen und abgeschnittene Dateien; keiner darf in Panik geraten.
* **Kein Raten:** Ein unsinniger Wert ist ein Fehler, der sagt welcher und warum, nie ein Ergebnis, das nur richtig aussieht.
* **Kein unsafe-Code:** `#![forbid(unsafe_code)]` in Bibliothek und Programm.

## 🔒 Sicherheit und Schutz

| Bedrohung | Schutz |
|---|---|
| Eine beschädigte, abgeschnittene oder leere Datei | keine Panik: die Tests schneiden gültige Eingaben an jeder Stelle ab und füttern Datenmüll |
| Eine unsinnige Zahl (negativ, nicht endlich, absurd groß) | wird mit einem Fehler abgelehnt, der den Wert nennt; nichts wird zu etwas Plausiblem gerundet |
| Unsichere Speichernutzung | reines sicheres Rust mit `forbid(unsafe_code)` |

**Was nicht abgesichert ist:** Ein fehlerfrei erzeugtes Ergebnis ist kein geprüftes Ergebnis. Nichts hier kennt die Grenzen einer Maschine, und das Programm hatte keine unabhängige Prüfung. Eine Schwachstelle bitte privat melden (siehe [SECURITY.md](SECURITY.md)).

## 🔌 Schnittstellen

Die Bibliothek ist `eh3d_vision_studio` (`use eh3d_vision_studio::...`); die Kommandozeile ist `eh3d-vision`. Die Befehle sind diese; jeder gibt sein Ergebnis auf der Standardausgabe aus und meldet einen Fehler auf der Fehlerausgabe mit einem Exit-Code ungleich null. Es gibt keinen Netzwerkzugriff.

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Kommandozeile
│   ├── align.rs                                 ein Teil der Logik
│   ├── cli_args.rs                              die Argumente der Kommandozeile, dieselbe Datei in jedem kleinen Werkzeug
│   ├── fiducial.rs                              ein Teil der Logik
│   └── scale.rs                                 ein Teil der Logik
├── tests/
│   ├── readme.rs                                Tests, die ausführen, was README und run-Skripte versprechen
│   ├── strict.rs                                Tests der Kommandozeile: Optionen und Wörter, die sie nicht kennt
│   └── vision.rs                                Tests von Bibliothek und Kommandozeile
├── examples/
│   ├── design.csv                               ein Beispiel zum Ausprobieren
│   ├── marks.pgm                                ein Beispiel zum Ausprobieren
│   └── measured.csv                             ein Beispiel zum Ausprobieren
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   └── structure.json                           Daten des Struktur-Bildes
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

## ⚙️ Konfiguration

Es gibt noch keine Konfigurationsdatei: Was die Person wählt, wird als Argument der Kommandozeile angegeben (siehe Schnittstellen), und jeder Standardwert steht in der Hilfe des Befehls (`eh3d-vision help`). Die festen Grenzen stehen als benannte Konstanten im Code und sind durch Tests abgedeckt.

## 🚀 Schnellstart

Sie brauchen eine stabile Rust-Toolchain (1.75 oder neuer). Das Repository EH3D-COMMON muss neben diesem geklont sein: der Bildleser wird daraus per Pfad übernommen.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
```

## 🧪 Tests

Die Suite hat 66 Tests (`cargo test`, in der CI unter Windows und Linux). Sie prüfen Normalfälle, Grenzfälle und beschädigte Eingaben: leere Dateien, Datenmüll und abgeschnittene Dateien lösen nie eine Panik aus. Die CI führt außerdem `cargo fmt --check`, `cargo clippy` mit Warnungen als Fehlern und `python -I tools/ci_validate.py` aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in `Cargo.toml` und in `eh3d.project.json`; jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` bauen das Programm, `test.bat` / `test.sh` führen die Tests aus und `run.bat` / `run.sh` starten es mit einem Beispiel. Releases erscheinen auf GitHub.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   die Bibliothek: was jeder Befehl nutzt
│   ├── main.rs                                  die Kommandozeile
│   ├── align.rs                                 ein Teil der Logik
│   ├── cli_args.rs                              die Argumente der Kommandozeile, dieselbe Datei in jedem kleinen Werkzeug
│   ├── fiducial.rs                              ein Teil der Logik
│   ├── image.rs                                 ein Teil der Logik
│   └── scale.rs                                 ein Teil der Logik
├── tests/
│   ├── readme.rs                                Tests, die ausführen, was README und run-Skripte versprechen
│   ├── strict.rs                                Tests der Kommandozeile: Optionen und Wörter, die sie nicht kennt
│   └── vision.rs                                Tests von Bibliothek und Kommandozeile
├── examples/
│   ├── design.csv                               ein Beispiel zum Ausprobieren
│   ├── marks.pgm                                ein Beispiel zum Ausprobieren
│   └── measured.csv                             ein Beispiel zum Ausprobieren
├── docs/
│   ├── architecture.json                        Daten des Architektur-Bildes
│   └── structure.json                           Daten des Struktur-Bildes
├── tools/                                       CI-Prüfung, README-Prüfung und die Bildwerkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Release-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── Cargo.toml Cargo.lock                        der Workspace und seine festgelegten Abhängigkeiten
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Es hat noch kein Fenster: nur Bibliothek und Kommandozeile existieren.
* Es wurde weder an einer Maschine noch mit einer Sammlung echter Dateien erprobt und hatte keine unabhängige Prüfung.
* Es liest eine Bilddatei, keine Live-Kamera, findet nur runde Marken (keine Kreuze oder Quadrate) und wurde an von den Tests gezeichneten Bildern erprobt, nicht an Fotos echter Platinen.

**Als Nächstes**

* Die Live-Kamera der Maschine lesen (EH3D-NODE-BUNGARD-AUX), Kreuze und Quadrate finden, Fotos echter Platinen ausprobieren und das Fenster mit dem Training neuer Marken ergänzen.
* Das Fenster im gemeinsamen Aussehen der Suite in den acht Sprachen ergänzen.
* Mit echten Dateien und den anderen Programmen der Suite ausprobieren.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): die gemeinsamen Crates, darunter der Bildleser
* [EH3D-PNP-MANAGER](https://github.com/JuanenRac/EH3D-PNP-MANAGER): die Pick-and-Place-Bestückung
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): das Isolationsfräsen von Kupfer
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): das Maschinensteuerungsprogramm
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): der Knoten des Bohrkopfs mit der Kamera
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): die Zentrale, die die Programme der Suite findet und startet

## 📚 Dokumentation

* [Changelog](CHANGELOG.md)
* [Architektur- und Strukturdaten](docs/architecture.json): die Daten, aus denen die beiden Bilder gezeichnet werden.

## 🤝 Mitwirken und Community

* Vor einem Pull Request `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` und `python -I tools/ci_validate.py` ausführen: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Die README gibt es in acht Sprachen: Eine Textänderung erfolgt in allen acht Dateien, und die zwei Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Eine Schwachstelle privat melden: [SECURITY.md](SECURITY.md). Fragen, Ideen und Berichte: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
