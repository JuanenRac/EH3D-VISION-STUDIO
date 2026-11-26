<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-VISION-STUDIO banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-VISION-STUDIO
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Fiduciali in un'immagine di telecamera: segni, scala e correzione di una scheda

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%246.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.1.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-functional-4CAF50.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-vision`) con 970 test eseguiti su un computer: trova segni fiduciali rotondi in un'immagine, converte i pixel in millimetri e calcola la rotazione e lo spostamento che portano una scheda dove dice il suo progetto. Ha una finestra (si compila con `--features gui`, il programma `eh3d-vision-gui`) che apre un'immagine, mostra i segni con le loro coordinate, imposta la scala, calcola la distorsione dell'obiettivo da una griglia, allinea al progetto ed esporta le opzioni di ISOCAM, ma la finestra non è ancora stata vista su uno schermo; non è stato provato su una macchina e non è stato verificato su una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.1.4` |
| Maturità | Funzionale |
| Test | 970 |
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
| Test | 🟡 | 970 test passano (`cargo test`), con casi normali, casi limite e input danneggiati, e una telecamera simulata sopra una scheda che aggiunge rumore, esposizione sbagliata, un fotogramma morto o assente, polvere che somiglia a un segno o un file immagine tagliato, e viene interrogata di nuovo. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`; un piccolo bordo unsafe solo nei motori di telecamera compilati con la propria funzionalità): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`, il programma `eh3d-vision-gui`): aprire o trascinare un'immagine, vedere i segni con le loro coordinate, impostare la scala con un segno di dimensione nota, calcolare la distorsione dell'obiettivo da una griglia, allineare a un disegno CSV ed esportare le opzioni di ISOCAM; la sua logica è testata, ma non è ancora stata compilata né vista su uno schermo. |
| Telecamera dal vivo | 🟠 | lo strato telecamera, la scacchiera di calibrazione e l'allineamento di una scheda sono scritti e provati su una telecamera e un tavolo simulati; i motori per Windows, V4L2 e libcamera sono scheletri e nulla è stato eseguito su una telecamera, un obiettivo o una scheda reali. |
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

## 🖼️ Schermate

<p align="center">
  <img src="images/screens/vision-studio-01-window.png" alt="La finestra con l'immagine e il disegno di esempio" width="100%">
</p>

<p align="center"><em>La finestra con l'immagine e il disegno di esempio: i due riferimenti trovati, ciascuno con il suo centro e il suo numero, e le impostazioni della ricerca a sinistra.</em></p>

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `eh3d-vision` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Riga di comando rigorosa:** un'opzione scritta male (`--powr`) o una parola in più è un errore che la nomina, non qualcosa da ignorare; altrimenti il programma farebbe la sua uscita con i valori predefiniti.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Limite di dimensione dei file letti:** la riga di comando legge un'immagine o un file di punti solo se non supera `max_file_mib` di `vision.cfg` (64 MiB di default); un file più grande è rifiutato con il codice `file_too_large` e il suo nome, uno illeggibile con `file_unreadable` e un file di punti che non è testo con `file_not_text`, e non si carica nulla.
* **Nessun codice unsafe nella compilazione predefinita:** `#![forbid(unsafe_code)]` nella libreria e nel programma; solo i motori delle telecamere dal vivo, compilati con la propria funzionalità, hanno un piccolo bordo unsafe (poche chiamate, con le dimensioni delle strutture verificate in compilazione).
* **I segni per ISOCAM:** `align --isocam-args` stampa ogni coppia di segni come l'opzione `--fiducial X_PROGETTO,Y_PROGETTO,X_MISURATA,Y_MISURATA` di `eh3d-isocam`.
* **Una finestra:** si compila con `--features gui` come programma `eh3d-vision-gui` (su Windows si apre senza console): apre un'immagine PNG o PGM con zoom e spostamento, trova i segni con valori modificabili, li disegna con le loro coordinate, imposta la scala con un segno di dimensione nota, calcola la distorsione dell'obiettivo da una griglia di segni, allinea a un disegno da un file CSV (i segni vengono abbinati per posizione, quindi l'ordine non conta) ed esporta le opzioni `--fiducial` di ISOCAM solo quando l'allineamento è accettato. Parla le otto lingue e non richiede la rete.
* **Un file di configurazione:** `vision.cfg` contiene ogni valore regolato (com'è fatto un segno, la griglia del target, i limiti di un allineamento accettato, il file più grande che si legge); un errore al suo interno viene rifiutato con la sua riga e `examples/vision.cfg` li elenca tutti.
* **Il centro a una frazione di pixel, con una confidenza:** `find --refine` (o `refine_subpixel` in `vision.cfg`) pesa i pixel grigi del bordo di ogni segno, così il suo centro si trova a una frazione di pixel, e stampa una confidenza che scende quando il segno si distingue appena dalla scheda.
* **Luce irregolare:** `--flatten PX` (o `flatten_window`) divide l'immagine per la luce attorno a ogni pixel prima della ricerca, così una lampada che si affievolisce sulla scheda o un'ombra non nascondono i segni del lato scuro.
* **Segni a forma di croce:** `eh3d-vision crosses IMMAGINE` trova segni più con i bracci lungo gli assi dell'immagine (riquadro quadrato, due bracci della stessa larghezza, angoli vuoti); dischi e quadrati vengono scartati e `--turned` trova croci con qualsiasi angolo (vedi sotto).
* **La scala dal passo di un target:** `eh3d-vision calibrate IMMAGINE --pitch-mm D` prende la mediana delle distanze tra segni vicini di una griglia, rifiuta un target non visto di fronte e stampa la riga `mm_per_px = ...` per `vision.cfg`.
* **Pezzi per altri programmi, in file semplici:** `eh3d-vision measure IMMAGINE --mm-per-px V` dà per ogni pezzo il centro (mm dal centro dell'immagine, x a destra, y in alto), la rotazione (da -90 escluso a 90 gradi), la lunghezza, la larghezza e l'area, in CSV (`index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2`) o in JSON (`--format json`: `format` `eh3d-vision-parts`, `version` 1, `unit`, `image_width_px`, `image_height_px`, `mm_per_px`, `parts`); col programma che li legge non si condivide altro. `--output FILE` non scrive mai sopra un file senza `--replace`, né sopra un collegamento.
* **Una cartella in un colpo:** `eh3d-vision batch CARTELLA --report FILE.csv` legge ogni immagine PNG e PGM entro il limite di dimensione (al massimo `batch_max_files`) e scrive una riga per immagine (`file,status,marks,mean_score,min_score,mean_diameter_px,detail`) con lo stato `ok`, `no_marks`, `picture_unreadable` o il codice del rifiuto; un nome di file che un foglio di calcolo eseguirebbe come formula viene scritto con un apice davanti.
* **Quadrati e croci con qualsiasi rotazione:** `eh3d-vision squares IMMAGINE` trova segni quadrati ruotati di un angolo qualsiasi (centro, lato e angolo antiorario come lo vede la macchina, da -45 escluso a 45 gradi) e `eh3d-vision crosses IMMAGINE --turned` trova croci i cui bracci non seguono gli assi; il riempimento minimo di un quadrato e il punteggio minimo di una croce (`square_min_fill`, `cross_min_score`) sono in `vision.cfg`.
* **Segni ad anello di una dimensione o di due:** `eh3d-vision rings IMMAGINE` trova segni rotondi con un foro (ciambelle): ognuno ha il suo centro, il diametro esterno e interno, il punteggio della forma, una fiducia (il punteggio abbassato quando l'anello si distingue poco dallo sfondo, `full_contrast`) e, quando gli anelli sono di due dimensioni, se è piccolo o grande (`--split PX` o `ring_split` dà il diametro che li separa, altrimenti si trova dove i diametri formano due gruppi separati da almeno `ring_size_gap`). Un disco, un anello con un'interruzione, un anello tagliato dal bordo, uno non rotondo e uno con il foro troppo piccolo o troppo grande (`ring_min_ratio`, `ring_max_ratio`) vengono scartati. La finestra ha una sezione per loro che disegna il cerchio esterno e quello interno con un colore per dimensione.
* **Dove è tenuto un pezzo prelevato:** `eh3d-vision offset IMMAGINE --package NOME` prende l'immagine di un pezzo visto dal basso (un solo pezzo), lo misura come `measure`, confronta la sua dimensione con il corpo del suo package come `inspect` e dà il suo scostamento dall'asse dell'ugello (`--nozzle-x`, `--nozzle-y` o `nozzle_x_mm`, `nozzle_y_mm`, millimetri dal centro dell'immagine, x a destra e y in alto), la sua rotazione rispetto a come dovrebbe essere tenuto (`--nominal-angle`; un pezzo lungo quanto largo ruota per la via più breve entro un quarto di giro, ogni altro entro mezzo giro) e la correzione che lo porta dove l'ugello lo vorrebbe: le stesse distanze e la rotazione al contrario. CSV o JSON, mai scritto sopra un file senza `--replace`; il codice di uscita è 1 quando il pezzo ha la dimensione sbagliata, è tenuto più lontano di `--max-offset` (`pick_max_offset_mm`) o ruotato più di `--max-turn` (`pick_max_turn_deg`), dopo aver scritto il file, e un'immagine senza pezzo o con più di un pezzo viene rifiutata (`no_parts`, `parts_count`). È geometria su un'immagine: non sono stati usati telecamera, ugello o macchina.
* **Una tabella di correzioni per un vassoio e un pannello della finestra per essa:** `eh3d-vision offset IMMAGINE --package NOME --each` prende ogni pezzo dell'immagine (un vassoio), ciascuno per conto suo, e scrive una tabella con la colonna `part` (il numero del pezzo, da 1) e poi le colonne di `offset`, una riga per pezzo; il codice di uscita è 1 quando un pezzo fallisce. La finestra ha un pannello per lo stesso (`nozzle_x_mm`, `nozzle_y_mm`, `nominal_angle_deg`, `pick_max_offset_mm`, `pick_max_turn_deg`): calcola la tabella dell'immagine aperta, elenca la correzione di ogni pezzo nella lingua della finestra e la salva come `<immagine>-offsets.csv`. `nominal_angle_deg` è la rotazione di un pezzo tenuto bene, anche in `vision.cfg`.
* **Una telecamera speculare e l'asse dell'ugello:** `--mirror` (`pick_mirror`) serve per una telecamera che vede i pezzi rovesciati da sinistra a destra: la x e la rotazione di ogni pezzo cambiano segno. `eh3d-vision nozzle IMMAGINE IMMAGINE... --package NOME` prende immagini di pezzi prelevati con lo stesso ugello (un pezzo in ciascuna, almeno due immagini) e scrive le righe `nozzle_x_mm` e `nozzle_y_mm` per `vision.cfg`, il punto medio dei pezzi (non dipende dall'asse in uso quando sono stati misurati); sull'uscita degli errori dice quanto sono dispersi i pezzi (la deviazione standard in x e in y, il pezzo più lontano e la rotazione media).
* **L'immagine basta per misurare:** `eh3d-vision quality IMMAGINE` dà la nitidezza (la varianza del laplaciano), il contrasto (la deviazione standard dei livelli di grigio) e il livello di grigio medio di un'immagine, in CSV o JSON, e la rifiuta (codice di uscita 1, dopo aver scritto il file) come `flat`, `dark`, `bright` o, quando `--min-sharpness` è sopra 0, `blurry`; i minimi sono `--min-contrast`, `--min-mean`, `--max-mean` e `--min-sharpness`, o le chiavi di `vision.cfg`. Con `quality_check = yes`, `offset`, `nozzle` e la finestra rifiutano un'immagine scadente (`poor_picture`) prima di misurarvi un pezzo, e la finestra ha un pannello per controllare l'immagine e mostrarne i numeri. I valori dipendono dalla telecamera e dall'obiettivo, quindi stanno nel file di configurazione e la nitidezza parte disattivata; nulla di questo è stato provato con una telecamera vera.
* **L'obiettivo con due fattori, rifiutato se non torna:** `eh3d-vision lens IMMAGINE` ricava `k1` e `k2` della distorsione radiale da una griglia di segni e scrive le righe `lens_k1 = ...` e `lens_k2 = ...` per `vision.cfg`; rifiuta un adattamento che lascia i segni a più di `lens_max_rms` pixel dal loro reticolo (`lens_residual_too_large`) o un fattore superiore a 0,5 (`lens_out_of_range`), e `find --undistort` raddrizza le posizioni con essi.
* **Un verdetto per ogni pezzo:** `eh3d-vision inspect IMMAGINE --package NOME --table packages.csv` misura i pezzi di un'immagine e confronta ciascuno con il corpo del suo contenitore (`package`, `body_long_mm`, `body_short_mm` e, se serve, `tolerance_mm`, in un file CSV; `examples/packages.csv` ne ha alcuni): `pass` o `fail` con il motivo (`length`, `width`, `angle`) in CSV o JSON, e il codice di uscita 1 quando un pezzo non passa; la tolleranza e la rotazione massima (`inspect_tolerance`, `inspect_max_angle`) sono in `vision.cfg`.
* **Un rapporto HTML della cartella:** `eh3d-vision batch CARTELLA --html rapporto.html` scrive una pagina che sta in piedi da sola (nessuno script, nessun collegamento, nessun altro file): una tabella delle immagini, un piccolo disegno di ciascuna con un cerchio su ogni segno trovato e gli avvisi (un'immagine non utilizzabile, senza segni o con un punteggio sotto `report_low_score`), nella lingua di `--lang` o di `vision.cfg`.
* **I nuovi strumenti nella finestra:** il pannello laterale ha il centro a una frazione di pixel con la fiducia di ogni segno, la finestra della luce, quadrati e croci con qualsiasi rotazione (disegnati sull'immagine), i pezzi in millimetri (salvati come `<immagine>-parts.csv`), il rapporto della cartella, letto in un thread con barra di avanzamento e pulsante di annullamento (`eh3d-vision-report.html`, mai scritto sopra un file esistente senza `replace_output`) e la scala dal passo di una mira, con i testi nelle otto lingue e i valori in `vision.cfg`; non è ancora stata guardata su uno schermo.
* **L'obiettivo nella finestra, con i due fattori:** il pulsante della griglia calcola `k1` e `k2` come fa `eh3d-vision lens` (rifiutato sopra `lens_max_rms`, un campo del pannello); la finestra disegna i punti in cui l'obiettivo calcolato mette il reticolo della griglia, da confrontare con i segni trovati (`show_lens_grid`), prende i fattori da `lens_k1` e `lens_k2` di `vision.cfg` senza griglia e li salva come `<immagine>-lens.cfg`.
* **Pezzi contro il loro package e immagini di un pezzo grande, nella finestra:** il pannello laterale legge la tabella dei package (`package_table`), giudica i pezzi misurati contro il package `inspect_package` (riga verde o rossa per pezzo, con il motivo del fallimento; `inspect_tolerance` e `inspect_max_angle` sono campi del pannello) e salva i verdetti come `<immagine>-inspection.csv`, il file di `eh3d-vision inspect`; pianifica inoltre le immagini che serve a un pezzo più grande del campo (il campo è l'immagine in millimetri, il pezzo è `shots_part_width` per `shots_part_height` o il corpo del package, con `shots_overlap` e `shots_margin`), elenca dove va la telecamera e salva `<immagine>-shots.csv`, il file di `eh3d-vision shots`. La riga di comando usa le stesse due nuove chiavi quando non sono dati né `--package` né la dimensione del pezzo.
* **Le immagini di un pezzo grande, pianificate:** `eh3d-vision shots --package NOME --fov-w MM --fov-h MM` (o `--body-long` e `--body-short`) elenca in CSV le posizioni della telecamera, in millimetri dal centro del pezzo, che vedono tutto un pezzo più grande del campo, con immagini vicine sovrapposte di `shots_overlap` e un margine di `shots_margin`; non serve nessuna telecamera.
* **Marcatori per correlazione:** `eh3d-vision correlate IMMAGINE --diameter PX` fa scorrere un disco di quel diametro sull'immagine e calcola in ogni punto la correlazione normalizzata (non cambia se la luce è più forte o il contrasto minore), così si trova un marcatore che la soglia perde con luce irregolare; i punti sono dati con la frazione di pixel e il punteggio, il migliore per primo, nessuno più vicino a uno migliore di un diametro. Richiede il diametro (`correlate_diameter`, `correlate_min_score`) ed è più lenta della ricerca a soglia, quindi un'immagine che richiederebbe troppo tempo viene rifiutata (`correlate_too_big`).
* **Prospettiva corretta:** `eh3d-vision rectify IMMAGINE --corners X1,Y1,...,X4,Y4 --output FILE.pgm` ridisegna l'immagine come sarebbe vista dall'alto, a partire dai quattro angoli di un rettangolo al suo interno (un'omografia con campionamento bilineare); angoli che non formano un quadrilatero convesso vengono rifiutati (`bad_corners`) e il lato maggiore è `rectify_max_side`.
* **Una fila di pad:** `eh3d-vision pads IMMAGINE --mm-per-px V` prende i pezzi dell'immagine come pad e dà il passo (la distanza mediana lungo la retta di miglior adattamento), quanto varia, quanto un pad si scosta dalla fila e la loro dimensione media, in CSV o JSON; il verdetto è `fail` con i motivi `pitch`, `row` e `expected` (`--expect-pitch`), rispetto a `pads_tolerance`, e il codice di uscita è 1.
* **Prima e dopo:** `eh3d-vision diff PRIMA DOPO` confronta due immagini della stessa dimensione: i pixel il cui livello di grigio differisce di più di `diff_threshold`, la loro quota, la differenza media e la massima, il riquadro che li racchiude e le regioni di almeno `diff_min_area` pixel; `--mask FILE.pgm` scrive l'immagine dei cambiamenti e il codice di uscita è 1 quando è cambiato più di `diff_max_changed`.
* **In che verso è tenuto un pezzo:** `eh3d-vision orientation IMMAGINE --mm-per-px V` trova il pezzo (ciò che differisce dalla scheda) e il segno che porta (la banda di un diodo, il punto del pin 1: ciò che differisce dal corpo al suo interno, anche se somiglia alla scheda) e dà la direzione dal centro del pezzo al segno (gradi in senso antiorario dall'asse x, y verso l'alto), il quarto di giro più vicino e quanto se ne discosta e, con `--expect-deg`, quanto si discosta dalla direzione voluta. Il pezzo è rifiutato (codice di uscita 1, dopo aver scritto il file) come `no_mark`, `centred`, `mark_big`, `wrong_way` o, senza direzione voluta, `skewed`.
* **L'impronta dei pad:** `eh3d-vision footprint IMMAGINE --pad-length MM --pad-width MM` misura ogni pad e lo giudica da solo rispetto al progetto con una tolleranza in millimetri (`--tolerance`, `footprint_tolerance_mm`): una riga per pad, dalla fila in alto verso il basso e da sinistra a destra, con le differenze e i motivi `length` e `width`; `--count` controlla anche il numero di pad (`count`) e il codice di uscita è 1 quando un pad è fuori.
* **Una mappa di dove l'immagine è nitida:** `eh3d-vision sharpmap IMMAGINE --cells N` taglia l'immagine in una griglia e dà la nitidezza di ogni cella (la varianza del laplaciano), la mediana, la cella più debole, l'inclinazione della nitidezza lungo x e lungo y e le celle deboli (sotto `--min-share` della mediana); una cella quasi senza contrasto è `empty` e non si giudica. `--heat FILE.pgm` scrive l'immagine della mappa, scura dove l'immagine è sfocata, e il codice di uscita è 1 quando una cella è debole (`uneven`).
* **La scala da una tavola di quadrati, con un rapporto:** `eh3d-vision board IMMAGINE --pitch-mm D --square-mm S` trova i quadrati, calcola la rotazione della tavola e la distanza tra vicini lungo ogni asse, e dà i millimetri di un pixel lungo x, lungo y e dal lato, con quanto sono quadrati i pixel (`aspect`) e quanto differiscono le distanze (`spread`); `--report FILE.txt` scrive il resoconto per una persona con la riga `mm_per_px = ...` per `vision.cfg`. È rifiutata (codice di uscita 1) quando i pixel non sono quadrati (`aspect`), le distanze differiscono (`spread`) o il lato e il passo non concordano (`disagree`).
* **Quattro pannelli in più nella finestra:** il pannello laterale ha la ricerca per correlazione (i segni trovati sono elencati con posto e punteggio), l'immagine raddrizzata con i quattro segni trovati (salvata come `<immagine>-rectified.pgm`), la fila di pad giudicata dai pezzi misurati (salvata come `<immagine>-pads.csv`) e il confronto dell'immagine aperta con un'altra (salvato come `<immagine>-diff.csv` e `<immagine>-changes.pgm`); i loro valori sono campi dei pannelli (`correlate_diameter`, `correlate_min_score`, `rectify_max_side`, `pads_tolerance`, `diff_threshold`, `diff_min_area`, `diff_max_changed`) e i loro testi sono in otto lingue. La loro logica è provata; la finestra non è stata vista su uno schermo.
* **Il server della macchina:** `eh3d-vision server health|config|discover|check|detect|send` usa le stesse chiavi degli altri programmi della suite (`server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns`) attraverso il client condiviso della suite (`eh3d-serverclient` di EH3D-COMMON): `config` legge i valori di `vision.cfg` che il server conserva per la macchina (l'artefatto più recente `vision-<machine>.cfg` del formato `text/x-eh3d-vision-config`, scritto come righe del file o come oggetto JSON; tutto o niente; le chiavi del collegamento non si prendono mai dal server) e li scrive come righe per il file, e `send quality|offset|pads|diff|orientation|footprint|sharpmap|board` conserva sul server il risultato JSON di quel comando come artefatto `vision-<kind>.json` (`POST /api/v1/artifacts`, con la macchina e il genere nei parametri). I nuovi tentativi e lo stato del collegamento sono quelli del client condiviso (una richiesta che può essere arrivata non viene mai inviata due volte), i rifiuti del server si possono dire con il loro codice in otto lingue, il token non viene mai stampato e si avvisa di un token che andrebbe in HTTP semplice verso un indirizzo non locale. Il server ha un plugin `vision`, quindi i risultati vengono conservati anche nella sua convenzione (i punti seguenti); si prova contro un server in memoria (`server::SimServer`) e un socket sul loopback, non contro il server vero.
* **Risultati nella convenzione del server:** `eh3d-vision server send fiducials|focus|blobs IMMAGINE` conserva un risultato nella forma che condividono il plugin `vision` di EH3D-SERVER e i programmi che guardano attraverso una telecamera: un file `vision-<tipo>.json` di formato `application/json` e schema `eh3d.vision.result/1`, con la dimensione e lo SHA-256 dell'immagine, i rilevamenti del tipo (`fiducials` `{x, y, radius_px, score}`, `focus` `{score, sharp}`, `blobs` `{x, y, area_px}`; `barcode` viene verificato ma mai prodotto, perché qui nulla legge codici), le unità (`px`) e l'evidenza (`picture`, salvo che `--evidence camera|simulator` dica altro: il programma non può sapere che un file venga da una telecamera). `server check FILE.json` dice se un documento segue la convenzione e nomina la parte che non la segue, e un documento che non la segue viene rifiutato prima dell'invio. Il punteggio di una verifica di messa a fuoco è 50 quando la nitidezza è `focus_reference`.
* **La stazione simulata del server:** `eh3d-vision server detect fiducials|barcode|focus|blobs IMMAGINE [--seed N] [--artifact]` chiede al plugin `vision` del server (`POST /api/v1/work/jobs`, avviato e letto fino alla fine) il risultato che inventa dalla dimensione e dall'hash dell'immagine e dal seme, sempre lo stesso (`evidence: simulator`), nella vista del lavoro o conservato come artefatto. `station::simulate` è un porting del plugin con cui risponde il server in memoria, così l'intera catena lavoro, risultato e artefatto è provata qui senza rete; non è stata eseguita contro il server vero.
* **La ricerca del server sulla rete, come vero interruttore:** `eh3d-vision server discover` elenca l'`host:porta` dei server che rispondono a `_eh3d._tcp` sulla rete locale. Non fa nulla finché `server_mdns` è spento (predefinito), e il programma deve essere compilato con `--features mdns` (quelli delle versioni lo sono) per avere il risolutore del client condiviso. Ciò che trova sono candidati tra cui la persona sceglie, mai connessi da soli, perché qualsiasi dispositivo della rete può rispondere.
* **Segni addestrati da un esempio, una scheda molto ruotata e un elenco sicuro di server:** `eh3d-vision train IMMAGINE --rect X,Y,W,H --output FILE` conserva la media di rettangoli ritagliati intorno a un segno come il file di un segno (mai scritto su un file che esiste, salvo con `--replace`), e `eh3d-vision locate --mark FILE IMMAGINE` lo ritrova per correlazione, ruotato passo passo se richiesto, con posizione, angolo e punteggio (le chiavi `train_*` di `vision.cfg`); `eh3d-vision pair PROGETTO.csv MISURATI.csv --any-turn` abbina i segni al disegno comunque sia ruotata la scheda, con un segno mancante o in più, e rifiuta ciò che due modi spiegano ugualmente bene (le chiavi `pair_*`); `discovery` trasforma ciò che risponde in rete in pochi candidati locali ben formati (le chiavi `find_server_*`), e `server discover` e la finestra elencano solo quelli. I comandi rispondono nella lingua di `vision.cfg` o di `--lang`, danno JSON con `--json` e terminano con il codice di uscita 0 (fatto), 1 (non si è potuto eseguire) o 2 (eseguito, ma il controllo non è stato superato: esempi che non concordano, un segno indistinguibile dai vicini, un segno non trovato, segni non abbinabili). Tutto è provato su immagini disegnate con la verità che le ha disegnate; il ciclo guardare, muovere, guardare ancora abbina ancora per posizione e la finestra non addestra ancora segni.
* **Un lettore di fotogrammi JPEG in movimento, una scheda di punti da stampare, la luce di un'immagine e i fori di un nastro:** `eh3d-vision jpeg FOTOGRAMMA` legge un fotogramma JPEG di base fino alla luminosità entro `mjpeg_max_bytes` e `mjpeg_max_pixels`, e un fotogramma tagliato, con un marcatore dove non può essercene uno, una tabella impossibile o una dimensione di miliardi di pixel viene rifiutato con un codice che non cambia (`jpeg_truncated`, `jpeg_bad_marker`, `jpeg_bad_table`, `jpeg_bad_size`, `jpeg_unsupported`, `jpeg_bad_data`), mai con un panico; `eh3d-vision calib-card` disegna una scheda di punti tondi in millimetri da stampare al 100 %, uguale ogni volta e rifiutata se non sta nella pagina (le chiavi `card_*`); `eh3d-vision light-check IMMAGINE` dice se la luce di un'immagine è buona (media, quota bruciata, zone, inclinazione; le chiavi `light_*`) e perché no, con il codice di uscita 2 se non lo è; `eh3d-vision tape-locate IMMAGINE` trova i fori di un nastro di componenti, il passo rispetto al passo del nastro e le tasche (le chiavi `tape_*`). Tutto è provato su immagini e fotogrammi disegnati con un seme fisso: non è stata usata alcuna telecamera, non è stata stampata alcuna scheda né fotografato alcun nastro.
* **Un bordo a una frazione di pixel, con la sua incertezza:** `eh3d-vision edge IMMAGINE --from X,Y --to X,Y` legge l'immagine lungo una linea tracciata attraverso un bordo (bilineare, a distanza `edge_step_px`), prende i due capi della linea come i due fondi piani, trova dove il profilo incrocia il livello a metà e dà la posizione lungo la linea e nell'immagine, il contrasto, la larghezza della transizione e l'incertezza (una deviazione standard: il rumore dei fondi, con l'arrotondamento a 8 bit aggiunto, diviso per la pendenza dove incrocia). Il codice di uscita è 1 quando l'incertezza supera `edge_max_uncertainty_px`; una linea senza bordo è `edge_flat`. Un bordo più netto di un pixel può inclinarsi fino a 0,05 px.
* **L'istogramma e la soglia automatica:** `eh3d-vision histogram IMMAGINE --bins N` dà i livelli di grigio in 8-256 gruppi e i tagli di Otsu (quello che usa la ricerca dei segni), di isodata e del metodo del triangolo, quanta varianza spiegano i due gruppi di Otsu, la quota di pixel neri e bianchi e se i tre tagli concordano (entro 8 livelli); se no, conviene controllare l'illuminazione o il `threshold` di `vision.cfg`.
* **Un'immagine contro un riferimento, attraverso una maschera di tolleranza:** `eh3d-vision refcheck RIFERIMENTO IMMAGINE --tolerance MASCHERA.pgm` confronta due immagini della stessa dimensione dove la differenza ammessa può cambiare da punto a punto: un pixel della maschera a 0 mantiene il `diff_threshold` generale, da 1 a 254 ammette altrettanti livelli di grigio in più, e 255 non viene guardato. Dà i pixel fuori tolleranza, di quanto la superano, in quante regioni (`diff_min_area`) e dove; `--map FILE.pgm` li scrive come immagine e il codice di uscita è 1 oltre `diff_max_changed`.
* **I residui dell'obiettivo, segno per segno:** `eh3d-vision lens IMMAGINE --residuals` scrive, per ogni segno della griglia, quanto dista dal suo posto prima e dopo la correzione, il segno più lontano, le tre fasce di distanza dal centro e il rapporto tra la fascia esterna e quella interna: un rapporto ben sopra 1 dice che due termini radiali non bastano per l'obiettivo (o che il bersaglio non è piano).
* **Di più nella finestra:** il pannello laterale ha le impostazioni del server (il pannello della suite di `eh3d-view`: indirizzo, token, macchina, tentativi, l'interruttore della ricerca sulla rete, accesso e sessioni), lo stato del collegamento nell'angolo, la ricerca dei server (solo con l'interruttore acceso, in un thread, con candidati da scegliere), un istogramma con i tre tagli, il bordo lungo una linea di quattro numeri, la verifica contro un riferimento con maschera di tolleranza e una riga di aiuto chiusa sotto i titoli dei pannelli, nelle otto lingue. La finestra non è stata guardata su uno schermo.
* **Test a seme fisso:** `tests/fuzz.rs` fornisce immagini di qualsiasi dimensione, angoli ovunque, pad di qualsiasi dimensione, quadrati in qualsiasi posto e file di configurazione, JSON, Base64 e risposte HTTP danneggiati, da un generatore che non legge l'orologio; niente deve andare in panic e ciò che esce deve avere senso, e i pannelli della finestra attraversano allo stesso modo immagini casuali. `tests/fuzz_camera.rs` fa lo stesso per un bordo lungo qualsiasi linea di qualsiasi immagine, l'istogramma, un'immagine contro un riferimento con qualsiasi maschera, i residui di segni qualsiasi e documenti della convenzione tagliati, alterati o riempiti di spazzatura.
* **Una telecamera dal vivo dietro un trait, provata solo con una simulata (`src/capture/`):** le telecamere sono elencate per nome con una chiave che non cambia (`vendor:product:serial@port`), aperte in un formato (YUYV, JPEG in movimento, RGB, grigio) e danno fotogrammi con un contatore e i loro byte originali; ciascuna delle 13 proprietà ha il suo intervallo o la telecamera dice di non averla. Si rilevano il carico di un bus USB (`capture_usb_mbps`), i fotogrammi neri, ripetuti, persi o tagliati e lo sfarfallio della luce, si attendono fotogrammi nuovi contandoli e non a secondi, e il registro va a una funzione. La telecamera simulata si guasta apposta; i motori per Windows, V4L2 e libcamera sono scheletri dietro le funzionalità `mfoundation`, `v4l2` e `libcamera` (spente per impostazione predefinita) e non sono mai stati provati su una telecamera reale, e i fotogrammi JPEG in movimento sono controllati ma non ancora decodificati.
* **Una scacchiera di calibrazione con marcatori numerati (`src/charuco/`):** `eh3d-vision charuco card` disegna una scacchiera le cui caselle bianche portano marcatori, in millimetri da stampare al 100 %; `eh3d-vision charuco calibrate IMAGE` legge i marcatori, trova gli angoli a una frazione di pixel con qualsiasi rotazione e adatta la mappa da pixel a millimetri in tre modi (omografia, affine, similitudine). Viene rifiutata se lascia troppo errore sugli angoli (`calib_max_residual_px`), se i metodi mettono gli angoli troppo distanti (`calib_max_method_gap`) o se non corrisponde alla distanza misurata con un righello tra i due angoli esterni di una riga (`calib_check_mm`); senza righello non si usa se non lo consente `calib_allow_unchecked`. L'unico scostamento è quello di `camera_offset_x_mm`, `camera_offset_y_mm` e `camera_offset_deg`, e una calibrazione di immagini simulate è rifiutata per una telecamera reale.
* **Una scheda rimessa a posto guardando, muovendo e guardando di nuovo (`src/boardaim.rs`):** i fiduciali sono guardati da una posa che non cambia, si calcolano rotazione e traslazione e il tavolo sposta la scheda del contrario (mai più di `align_max_step_mm` e `align_max_step_deg` per volta, né `align_max_total_mm` in tutto); dopo aver atteso fotogrammi presi dopo lo spostamento si guarda di nuovo, finché la scheda è entro `align_tolerance_mm` e `align_tolerance_deg`. Quando qualcosa va storto si va solo verso lo stato più sicuro (passi interi, mezzi passi, fermo, abbandono): un errore che cresce dimezza il passo e poi si ferma, e l'assenza di segni, fotogrammi neri o ripetuti, un limite o un guasto del tavolo fermano i movimenti. È stata eseguita solo con un tavolo simulato.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Un rapporto o un file di misure che scriverebbe sopra un altro file | si scrive solo dove non ce n'è uno; per scrivere sopra serve `--replace`, e non si scrive mai attraverso un collegamento |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)`; un piccolo bordo unsafe solo nei motori di telecamera compilati con la propria funzionalità |
| Un token mostrato o inviato dove non deve | il token non viene mai stampato né messo in un risultato; i valori che invia un server si prendono tutti o nessuno e le chiavi del collegamento (`server_*`) non si prendono mai da lui; si avvisa di un token che andrebbe in HTTP semplice verso un indirizzo non locale |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_vision_studio` (`use eh3d_vision_studio::...`); la riga di comando è `eh3d-vision`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Solo il comando `server` usa la rete (HTTP semplice verso il server della macchina indicato in `vision.cfg`); tutti gli altri lavorano su file locali.

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

I comandi `train`, `locate`, `pair`, `light-check` e `tape-locate` terminano con il codice di uscita 0 quando sono fatti, 1 quando non si sono potuti eseguire (un'opzione sbagliata, un file illeggibile o non scrivibile, un limite superato) e 2 quando sono stati eseguiti ma il loro controllo non è stato superato.

## ⚙️ Configurazione

I valori regolati stanno in un semplice file `chiave = valore`, non nel codice: `vision.cfg` nella cartella da cui parte il programma o accanto ad esso (la variabile `EH3D_VISION_CONFIG` indica un altro file). Fissa la lingua della finestra, se i segni sono scuri o chiari, la soglia, l'intervallo del diametro, il riempimento e l'allungamento di un segno, la dimensione del segno che imposta la scala, le colonne e le righe della griglia, se l'allineamento può cambiare la dimensione, i limiti che un allineamento deve rispettare e il file più grande che si legge. `examples/vision.cfg` elenca ogni chiave con il suo significato e il valore con cui parte il programma; se il file manca valgono quei valori e un file con un errore viene rifiutato con la sua riga. Sulla riga di comando le stesse cose si danno come opzioni (vedi Interfacce). Contiene anche gli altri valori regolati: la confidenza (`refine_subpixel`, `full_contrast`), la finestra della luce (`flatten_window`), la scala e la dimensione di un pezzo (`mm_per_px`, `part_min_area`, `part_max_area`, `max_parts`), l'intervallo di una croce (`cross_min_size`, `cross_max_size`, `cross_max_elongation`), la tolleranza di un target (`pitch_tolerance`) e il massimo di immagini di una cartella (`batch_max_files`). Il riempimento minimo di un quadrato e il punteggio minimo di una croce con qualsiasi rotazione sono `square_min_fill` e `cross_min_score`. L'intervallo del lato di un quadrato e di quanto la sua scatola può essere più lunga che larga sono `square_min_size`, `square_max_size` e `square_max_elongation`, separati da quelli della croce. L'obiettivo è `lens_k1` e `lens_k2` (come li ricava `lens`) e lo scarto massimo di una griglia dal suo reticolo dopo la correzione è `lens_max_rms`. La tabella dei contenitori che legge `inspect` è `package_table`, e lo scarto massimo di un pezzo dal suo contenitore e la sua rotazione massima sono `inspect_tolerance` e `inspect_max_angle`. Il punteggio sotto il quale il rapporto HTML avvisa su un'immagine è `report_low_score`. La distanza tra i segni della mira che fissa la scala è `pitch_mm`, e `replace_output` dice se i file scritti possono esserlo sopra un file esistente (no per impostazione predefinita). La sovrapposizione delle immagini vicine di un piano di `shots` e il margine intorno al pezzo sono `shots_overlap` e `shots_margin`. Se la finestra disegna il reticolo dell'obiettivo lo dice `show_lens_grid`. Il package contro cui `inspect` e la finestra giudicano i pezzi è `inspect_package`, e la dimensione del pezzo per cui `shots` e la finestra pianificano le immagini è `shots_part_width` e `shots_part_height` (0 e 0: il corpo del package). I segni ad anello hanno l'intervallo del diametro esterno (`ring_min_diameter`, `ring_max_diameter`), del rapporto tra diametro interno ed esterno (`ring_min_ratio`, `ring_max_ratio`), il riempimento minimo della banda (`ring_min_fill`), l'allungamento (`ring_max_elongation`), il diametro che separa le due dimensioni (`ring_split`, 0 per trovarlo) e il divario tra le dimensioni (`ring_size_gap`). L'asse dell'ugello nell'immagine di un pezzo prelevato è `nozzle_x_mm` e `nozzle_y_mm`, e la distanza massima a cui può essere tenuto e la rotazione massima sono `pick_max_offset_mm` e `pick_max_turn_deg` (0: non controllato). La rotazione di un pezzo tenuto bene è `nominal_angle_deg`, se la telecamera vede i pezzi speculari è `pick_mirror`, e il controllo dell'immagine è `quality_check` con i minimi `quality_min_contrast`, `quality_min_mean`, `quality_max_mean` e `quality_min_sharpness` (0: non controllato). Il server è `server_address`, `server_token`, `server_machine`, `server_timeout_s` e `server_retries` (indirizzo vuoto: nessun server); la ricerca per correlazione è `correlate_diameter` e `correlate_min_score`; il lato maggiore di un'immagine raddrizzata è `rectify_max_side`; la tolleranza di una fila di pad è `pads_tolerance`; e il confronto di due immagini è `diff_threshold`, `diff_min_area` e `diff_max_changed`. Se il server si cerca sulla rete locale lo dice `server_mdns` (spento per impostazione predefinita). La lettura di un pezzo con un segno è `orientation_body_contrast`, `orientation_mark_contrast`, `orientation_min_mark_px` e `orientation_tolerance_deg`; la tolleranza di un pad è `footprint_tolerance_mm`; la mappa di nitidezza è `sharpmap_cells`, `sharpmap_min_share` e `sharpmap_min_contrast`; e la scala da una tavola è `board_square_mm` (la distanza tra i centri è `pitch_mm`), `board_max_aspect` e `board_max_spread`. Un `vision.cfg` che contiene un token va tenuto privato. La nitidezza che vale 50 punti in un risultato di messa a fuoco è `focus_reference`, e il passo lungo la linea di un bordo e la massima incertezza della sua posizione sono `edge_step_px` e `edge_max_uncertainty_px`. La telecamera dal vivo, la scacchiera di calibrazione e l'allineamento di una scheda hanno chiavi proprie (`capture_*`, `calib_*`, `camera_offset_*` e `align_*`): ognuna ha un intervallo, un valore fuori è rifiutato e un errore governato da una chiave ne cita il nome. Il lettore di fotogrammi JPEG in movimento, la scheda di punti, il controllo della luce e la ricerca dei fori di un nastro hanno chiavi proprie (`mjpeg_*`, `card_*`, `light_*` e `tape_*`): ognuna ha un intervallo, e un valore fuori da esso viene rifiutato nominando la chiave.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente). Il repository EH3D-COMMON deve essere clonato accanto a questo: il lettore di immagini viene preso da lì tramite percorso.

```bash
cargo build --release
cargo test
cargo run --release -- align examples/design.csv examples/measured.csv
cargo run --release --features gui --bin eh3d-vision-gui   # the window
cargo run --release --features gui --bin eh3d-vision-gui -- --shot screen.png --open examples/marks.pgm --open examples/design.csv   # a picture of the window
```

Per fare un'immagine della finestra senza aprirla a mano, eseguila con `--shot FILE.png` (anche `--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace`, `--mark N`, `--align`): si apre con quella dimensione, salva il PNG, non conserva impostazioni e si chiude da sola (codice di uscita 0 se il file è stato scritto, 1 se no, 2 per argomenti errati). Con `--mark N` il segno numero N (da 1) fissa la scala e con `--align` il disegno viene allineato ai segni, così l'immagine mostra le linee dell'allineamento: dal punto in cui il disegno colloca ogni segno al segno trovato, con la distanza in millimetri (`show_residuals` in `vision.cfg` le disattiva). `examples/design_pair.csv` è un disegno per i due segni di `examples/marks.pgm`.

## 🧪 Test

La suite ha 970 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

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
│   ├── batch.rs                                 una cartella di immagini in un colpo, con un rapporto CSV
│   ├── board.rs                                 la scala da una tavola di quadrati: lungo x, lungo y e dal lato, con rapporto
│   ├── boardaim.rs                              una scheda messa dove dice il disegno: guardare, muovere, guardare, sempre verso il più sicuro
│   ├── capture/                                 la telecamera dal vivo: trait, formati, proprietà, budget USB, salute del flusso, simulata
│   ├── charuco/                                 la scacchiera di calibrazione con marcatori numerati: angoli, calibrazione, file e scheda
│   ├── config.rs                                il file di configurazione: ogni valore regolato
│   ├── correlate.rs                             marcatori rotondi trovati per correlazione con un disco del diametro voluto
│   ├── diff.rs                                  due immagini dello stesso punto confrontate: cosa è cambiato, dove e di quanto
│   ├── discovery.rs                             risposte della rete in un breve elenco di candidati sicuri, e un risolutore a copione
│   ├── cross.rs                                 marcatori fiduciali a forma di croce
│   ├── edge.rs                                  un bordo a una frazione di pixel lungo una linea, con la sua incertezza
│   ├── fiducial.rs                              i riferimenti rotondi di un'immagine: soglia, macchie e controlli
│   ├── flatten.rs                               la luce irregolare tolta da un'immagine
│   ├── footprint.rs                             ogni pad giudicato rispetto alla dimensione del progetto, con una tolleranza in millimetri
│   ├── grid.rs                                  la distorsione dell'obiettivo da una griglia di riferimenti
│   ├── inspect.rs                               ogni pezzo confrontato con il corpo del suo contenitore da una tabella, con verdetto
│   ├── job.rs                                   il rapporto di una cartella letto in un thread, con avanzamento e annullamento
│   ├── lens.rs                                  i due fattori dell'obiettivo da una griglia di segni, rifiutati se non tornano
│   ├── points.rs                                i file di punti di un disegno
│   ├── quality.rs                               se un'immagine basta per misurare: nitidezza, contrasto e livello medio
│   ├── reference.rs                             un'immagine contro un riferimento con una maschera di tolleranza
│   ├── histogram.rs                             l'istogramma e i tagli di Otsu, isodata e del metodo del triangolo
│   ├── report.rs                                il rapporto di una cartella come una pagina HTML autonoma, in otto lingue
│   ├── ring.rs                                  i segni ad anello: rotondi con un foro, di una o due dimensioni, con fiducia
│   ├── limit.rs                                 lettura di un file con limite di dimensione e un codice per ogni rifiuto
│   ├── measure.rs                               pezzi misurati in millimetri, file CSV e JSON
│   ├── offset.rs                                come è tenuto un pezzo prelevato, con correzione, tabella di un vassoio e asse dell'ugello
│   ├── orientation.rs                           in che verso è tenuto un pezzo con segno, dalla direzione del segno
│   ├── pads.rs                                  una fila di pad: il passo, quanto varia e quanto un pad si scosta dalla fila
│   ├── panels.rs                                i pannelli della finestra: obiettivo, package, piano immagini, scostamenti e controllo immagine
│   ├── perspective.rs                           prospettiva corretta: l'immagine ridisegnata vista dall'alto, da quattro angoli
│   ├── scale.rs                                 dai pixel ai millimetri e la distorsione radiale dell'obiettivo
│   ├── server.rs                                il percorso verso il server EH3D tramite il client condiviso e un server in memoria per i test
│   ├── convention.rs                            il risultato di visione nella convenzione del server: crearlo, verificarlo, SHA-256
│   ├── station.rs                               la stazione di visione simulata del server: il lavoro, la risposta, un porting
│   ├── sharpmap.rs                              una mappa di dove l'immagine è nitida: una griglia di celle, le deboli e l'inclinazione
│   ├── square.rs                                segni quadrati e croci ruotate di un angolo qualsiasi
│   ├── tools.rs                                 gli strumenti del pannello laterale sull'immagine: forme, pezzi, rapporto, scala dal passo
│   ├── train.rs                                 un segno addestrato da esempi, ritrovato per correlazione all'angolo in cui è ruotato
│   ├── turnpair.rs                              segni abbinati al disegno per una scheda ruotata di qualsiasi angolo
│   ├── dotcard.rs                               la scheda di punti tondi da stampare al 100 %, come disegno in millimetri
│   ├── light.rs                                 se la luce di un'immagine è buona: media, bruciato, zone e inclinazione
│   ├── mjpeg.rs                                 un lettore di fotogrammi JPEG in movimento che non si fida dei suoi byte
│   ├── tape.rs                                  i fori e le tasche di un nastro di componenti e il passo rispetto a quello del nastro
│   ├── verbs.rs                                 i comandi train, locate, pair, light-check, tape-locate, calib-card e jpeg
│   ├── shot.rs                                  la modalità immagine della finestra (--shot): i suoi argomenti, letti e verificati senza schermo
│   ├── shots.rs                                 le posizioni della telecamera che coprono un pezzo più grande del campo, dalle sole misure
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── checks.rs                                pannelli della finestra: correlazione, raddrizzamento, fila di pad, confronto
│   ├── readings.rs                              pannelli della finestra: istogramma, bordo, riferimento e impostazioni del server
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: immagine, riferimenti, scala, obiettivo e allineamento
├── tests/
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   ├── strict.rs                                test della riga di comando: opzioni e parole che non conosce
│   ├── read_limit.rs                            test del limite di dimensione dei file letti dalla riga di comando
│   ├── exchange.rs                              test dei comandi che leggono un'immagine e scrivono un file
│   ├── commands.rs                              test dei comandi correlate, rectify, pads, diff e server
│   ├── analysis.rs                              test dei comandi orientation, footprint, sharpmap e board
│   ├── capture.rs                               telecamera dal vivo, calibrazione e allineamento su quella simulata, e il comando
│   ├── fuzz.rs                                  test con input casuale e seme fisso: niente deve andare in panic
│   ├── camera.rs                                i nuovi comandi eseguiti come programma: edge, histogram, refcheck, residui, server
│   ├── fuzz_camera.rs                           input casuale con seme fisso per le nuove letture e la convenzione
│   ├── fuzz_capture.rs                          input casuale con seme fisso per lettori di fotogrammi, calibrazione, allineamento
│   ├── training.rs                              un segno addestrato ritrovato dove e come è stato disegnato, con i file
│   ├── pairing.rs                               segni di una scheda ruotata di qualsiasi angolo abbinati secondo la verità
│   ├── mjpeg.rs                                 fotogrammi codificati riletti, tagliati ovunque, byte cambiati, rumore
│   ├── newcli.rs                                i nuovi comandi: testo, JSON, file rispettati, codici di uscita, esempi della guida
│   ├── lightfuzz.rs                             luce, nastro e scheda con qualsiasi immagine e impostazione, seme fisso
│   ├── discovery.rs                             le risposte della rete, ostili o numerose, non danno mai un candidato non sicuro
│   ├── shot_sample.rs                           test che gli esempi si aprano nella finestra di un'immagine
│   ├── vision.rs                                test della libreria e della riga di comando
│   └── reqs.rs                                  test del registro dei requisiti
├── examples/
│   ├── design.csv                               un esempio per provare il programma
│   ├── design_pair.csv                          un disegno per i due segni di marks.pgm
│   ├── marks.pgm                                un esempio per provare il programma
│   ├── measured.csv                             un esempio per provare il programma
│   ├── packages.csv                             una tabella di corpi di contenitori per inspect
│   └── vision.cfg                               il file di configurazione con ogni valore e il suo significato
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   ├── NOT_YET.md                               buone idee fuori da questa versione, con il perché e il quando
│   ├── REQUIREMENTS.md                          il registro dei requisiti: stato, evidenza e test di ciascuno
│   ├── SCOPE.md                                 a cosa serve, cosa non farà e cosa non decide mai
│   ├── STATUS.md                                cosa esiste, cosa manca e cosa è ancora aperto
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── AGENTS.md                                    le regole per chi modifica il repository
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* La finestra non è ancora stata vista su uno schermo; `align` e il ciclo guardare, muovere, guardare ancora abbinano i segni al disegno per posizione, quindi lì una scheda molto ruotata o un segno mancante viene rifiutato (`pair --any-turn` li abbina).
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Legge file immagine, e una telecamera dal vivo solo in simulazione (i motori per Windows, V4L2 e libcamera sono scheletri mai provati su una telecamera reale), trova solo segni rotondi, quadrati e a forma di croce ed è stato provato su immagini disegnate dai test, non su fotografie di schede reali. La rotazione di un pezzo è misurata dai suoi pixel come per un rettangolo, quindi un pezzo che non è un rettangolo riceve la rotazione del suo asse lungo.
* Il server non ha un plugin di visione, quindi il programma usa gli artefatti del magazzino (`POST /api/v1/artifacts` per un risultato, `GET /api/v1/artifacts` e `GET /api/v1/artifacts/{id}` per la configurazione di una macchina) con nomi e un formato propri (`vision-<kind>.json`, `vision-<machine>.cfg`); oggi si prova solo contro un server in memoria e un socket sul loopback, e la ricerca del server con mDNS (`server_mdns`) è un interruttore senza risolutore collegato.

**Prossimi passi**

* Leggere la telecamera dal vivo della macchina (EH3D-NODE-BUNGARD-AUX), provare fotografie di schede reali e mettere nella finestra l'addestramento di nuovi segni, che i comandi hanno già.
* Vedere la finestra su uno schermo e finirla.
* Provarlo con file reali e con gli altri programmi della suite.
* Scrivere i motori dal vivo (fotogrammi da Media Foundation, V4L2 e libcamera), provare la scacchiera di calibrazione con un obiettivo reale, la scheda di punti su carta e l'allineamento con un tavolo reale.

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
* [Ambito](docs/SCOPE.md): a cosa serve il programma, cosa non farà e cosa non decide mai.
* [Requisiti](docs/REQUIREMENTS.md): il registro delle capacità, lo stato di ciascuna e il test che la mostra.
* [Stato](docs/STATUS.md): cosa esiste, cosa manca e cosa è ancora aperto.
* [Non ancora](docs/NOT_YET.md): buone idee lasciate fuori da questa versione.
* [Regole per modificare il repository](AGENTS.md): le regole per chi modifica il repository.
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
