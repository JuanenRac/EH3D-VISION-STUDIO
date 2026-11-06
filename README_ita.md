<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciali in un'immagine di telecamera: segni, scala e correzione di una scheda

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-vision`) con 138 test eseguiti su un computer: trova segni fiduciali rotondi in un'immagine, converte i pixel in millimetri e calcola la rotazione e lo spostamento che portano una scheda dove dice il suo progetto. Ha una finestra (si compila con `--features gui`, il programma `eh3d-vision-gui`) che apre un'immagine, mostra i segni con le loro coordinate, imposta la scala, calcola la distorsione dell'obiettivo da una griglia, allinea al progetto ed esporta le opzioni di ISOCAM, ma la finestra non è ancora stata vista su uno schermo; non è stato provato su una macchina e non è stato verificato su una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.3` |
| Maturità | Impalcatura |
| Test | 138 |
| Stack tecnologico | Rust |
| Su hardware reale | Non provato: nulla di questo ha girato su una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Nucleo | 🟡 | trova segni fiduciali rotondi in un'immagine, converte i pixel in millimetri e calcola la rotazione e lo spostamento che portano una scheda dove dice il suo progetto; provato su un computer con input normali e danneggiati, non con una raccolta di file reali. |
| Riga di comando | 🟡 | `eh3d-vision` esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 138 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`, il programma `eh3d-vision-gui`): aprire o trascinare un'immagine, vedere i segni con le loro coordinate, impostare la scala con un segno di dimensione nota, calcolare la distorsione dell'obiettivo da una griglia, allineare a un disegno CSV ed esportare le opzioni di ISOCAM; la sua logica è testata, ma non è ancora stata compilata né vista su uno schermo. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **I segni** (`src/fiducial.rs`): l'immagine è divisa in scuro e chiaro con la soglia di Otsu, le macchie connesse sono misurate e quelle che sono dischi della dimensione attesa (l'area, quanto riempiono il cerchio che le contiene e quanto sono allungate) sono i segni, i migliori per primi; una macchia tagliata dal bordo dell'immagine è esclusa perché il suo centro non è affidabile, e i segni possono essere scuri su chiaro o chiari su scuro.
* **La scala e l'obiettivo** (`src/scale.rs`): i millimetri di un pixel vengono da un segno di dimensione nota, gli assi della macchina salgono mentre l'immagine scende, e la distorsione radiale dell'obiettivo si ricava ai minimi quadrati da una griglia di segni e si corregge.
* **La scheda** (`src/align.rs`): da due o più fiduciali del progetto e gli stessi come li ha trovati la telecamera, si calcolano ai minimi quadrati la rotazione, lo spostamento e (se consentito) la dimensione, con il residuo in millimetri; una trasformazione con residuo troppo grande (un segno sbagliato) o dimensione deviata (una scheda stirata) non è accettata.

## 🧩 Ruolo in EH3D

EH3D-VISION-STUDIO è un programma della EH3D Fabrication Suite. Funziona come libreria e riga di comando e, compilato con `--features gui`, come finestra con l'aspetto comune agli altri programmi della suite. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-VISION-STUDIO: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `eh3d-vision` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Riga di comando rigorosa:** un'opzione scritta male (`--powr`) o una parola in più è un errore che la nomina, non qualcosa da ignorare; altrimenti il programma farebbe la sua uscita con i valori predefiniti.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Nessun codice unsafe:** `#![forbid(unsafe_code)]` nella libreria e nel programma.
* **I segni per ISOCAM:** `align --isocam-args` stampa ogni coppia di segni come l'opzione `--fiducial X_PROGETTO,Y_PROGETTO,X_MISURATA,Y_MISURATA` di `eh3d-isocam`.
* **Una finestra:** si compila con `--features gui` come programma `eh3d-vision-gui` (su Windows si apre senza console): apre un'immagine PNG o PGM con zoom e spostamento, trova i segni con valori modificabili, li disegna con le loro coordinate, imposta la scala con un segno di dimensione nota, calcola la distorsione dell'obiettivo da una griglia di segni, allinea a un disegno da un file CSV (i segni vengono abbinati per posizione, quindi l'ordine non conta) ed esporta le opzioni `--fiducial` di ISOCAM solo quando l'allineamento è accettato. Parla le otto lingue e non richiede la rete.
* **Un file di configurazione:** `vision.cfg` contiene ogni valore regolato (com'è fatto un segno, la griglia del target, i limiti di un allineamento accettato, il file più grande che si legge); un errore al suo interno viene rifiutato con la sua riga e `examples/vision.cfg` li elenca tutti.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)` |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_vision_studio` (`use eh3d_vision_studio::...`); la riga di comando è `eh3d-vision`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Non c'è accesso alla rete.

```text
eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]
eh3d-vision-gui [IMAGE] [DESIGN.csv]   the window (a build with --features gui); on Windows it opens with no console
```

## ⚙️ Configurazione

I valori regolati stanno in un semplice file `chiave = valore`, non nel codice: `vision.cfg` nella cartella da cui parte il programma o accanto ad esso (la variabile `EH3D_VISION_CONFIG` indica un altro file). Fissa la lingua della finestra, se i segni sono scuri o chiari, la soglia, l'intervallo del diametro, il riempimento e l'allungamento di un segno, la dimensione del segno che imposta la scala, le colonne e le righe della griglia, se l'allineamento può cambiare la dimensione, i limiti che un allineamento deve rispettare e il file più grande che si legge. `examples/vision.cfg` elenca ogni chiave con il suo significato e il valore con cui parte il programma; se il file manca valgono quei valori e un file con un errore viene rifiutato con la sua riga. Sulla riga di comando le stesse cose si danno come opzioni (vedi Interfacce).

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente). Il repository EH3D-COMMON deve essere clonato accanto a questo: il lettore di immagini viene preso da lì tramite percorso.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Per fare un'immagine della finestra senza aprirla a mano, eseguila con `--shot FILE.png` (anche `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`): si apre con quella dimensione, salva il PNG, non conserva impostazioni e si chiude da sola (codice di uscita 0 se il file è stato scritto, 1 se no, 2 per argomenti errati).

## 🧪 Test

La suite ha 138 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

### Download

Ogni versione contiene il programma con la sua finestra per Windows (64 e 32 bit), Linux (64 bit) e macOS (Apple silicon e Intel), come `<nome>-<versione>-<sistema>.zip` (Windows) o `.tar.gz` con il programma, la licenza, il leggimi, l'icona e gli esempi. I file sono costruiti da GitHub Actions, elencati in `SHA256SUMS` e firmati con la chiave di EH3D. Per controllare un download, confronta il suo hash con il file `.sha256` accanto ad esso (`sha256sum -c`) e verifica la firma con il file `.sig` accanto ad esso (`ssh-keygen -Y verify`).

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-VISION-STUDIO" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-VISION-STUDIO/
├── src/
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── gui_main.rs                              la finestra come programma a sé (funzione gui)
│   ├── cli_args.rs                              gli argomenti della riga di comando, lo stesso file in ogni piccola utilità
│   ├── align.rs                                 la rotazione, lo spostamento e la dimensione di una scheda e il suo abbinamento al disegno
│   ├── config.rs                                il file di configurazione: ogni valore regolato
│   ├── fiducial.rs                              i riferimenti rotondi di un'immagine: soglia, macchie e controlli
│   ├── grid.rs                                  la distorsione dell'obiettivo da una griglia di riferimenti
│   ├── points.rs                                i file di punti di un disegno
│   ├── scale.rs                                 dai pixel ai millimetri e la distorsione radiale dell'obiettivo
│   ├── shot.rs                                  la modalità immagine della finestra (--shot): i suoi argomenti, letti e verificati senza schermo
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: immagine, riferimenti, scala, obiettivo e allineamento
├── tests/
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   ├── strict.rs                                test della riga di comando: opzioni e parole che non conosce
│   ├── shot_sample.rs                           test che gli esempi si aprano nella finestra di un'immagine
│   └── vision.rs                                test della libreria e della riga di comando
├── examples/
│   ├── design.csv                               un esempio per provare il programma
│   ├── marks.pgm                                un esempio per provare il programma
│   ├── measured.csv                             un esempio per provare il programma
│   └── vision.cfg                               il file di configurazione con ogni valore e il suo significato
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* La finestra non è ancora stata vista su uno schermo; i segni vengono abbinati al disegno per posizione, quindi una scheda molto ruotata o un segno mancante viene rifiutato invece di essere indovinato.
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Legge un file immagine, non una telecamera dal vivo, trova solo segni rotondi (né croci né quadrati) ed è stato provato su immagini disegnate dai test, non su fotografie di schede reali.

**Prossimi passi**

* Leggere la telecamera dal vivo della macchina (EH3D-NODE-BUNGARD-AUX), trovare croci e quadrati, provare fotografie di schede reali e aggiungere alla finestra l'addestramento di nuovi segni.
* Vedere la finestra su uno schermo e finirla.
* Provarlo con file reali e con gli altri programmi della suite.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): i crate condivisi, tra cui il lettore di immagini
* [EH3D-PHOENIX-PNP](https://github.com/JuanenRac/EH3D-PHOENIX-PNP): l'assemblaggio pick and place
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): la fresatura di isolamento del rame
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): il programma di controllo della macchina
* [EH3D-NODE-BUNGARD-AUX](https://github.com/JuanenRac/EH3D-NODE-BUNGARD-AUX): il nodo della testa di foratura con la telecamera
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): il centro che trova e avvia i programmi della suite

## 📚 Documentazione

* [Changelog](CHANGELOG.md)
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
