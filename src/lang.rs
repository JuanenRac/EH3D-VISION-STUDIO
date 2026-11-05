//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }

    /// The language that the configuration asks for (`auto` is the one of the system).
    pub fn from_setting(setting: &str) -> Lang {
        if setting == "auto" {
            Lang::from_env()
        } else {
            Lang::from_code(setting).unwrap_or(Lang::En)
        }
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill<S: AsRef<str>>(template: &str, values: &[S]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v.as_ref());
    }
    out
}

/// Makes `Msg` (every word and sentence of the window) with its list of texts, in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
macro_rules! messages {
    ($($name:ident => [$en:expr, $es:expr, $de:expr, $fr:expr, $it:expr, $ja:expr, $zh:expr, $pt:expr,],)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Msg {
            $($name),*
        }

        impl Msg {
            /// Every sentence, once.
            pub const ALL: &'static [Msg] = &[$(Msg::$name),*];

            fn words(self) -> [&'static str; 8] {
                match self {
                    $(Msg::$name => [$en, $es, $de, $fr, $it, $ja, $zh, $pt]),*
                }
            }
        }
    };
}

messages! {
    OpenImage => [
        "Open picture...",
        "Abrir imagen...",
        "Bild öffnen...",
        "Ouvrir une image...",
        "Apri immagine...",
        "画像を開く...",
        "打开图像...",
        "Abrir imagem...",
    ],
    OpenDesign => [
        "Load design (CSV)...",
        "Cargar diseño (CSV)...",
        "Entwurf laden (CSV)...",
        "Charger le dessin (CSV)...",
        "Carica il disegno (CSV)...",
        "設計を読み込む (CSV)...",
        "载入设计 (CSV)...",
        "Carregar o desenho (CSV)...",
    ],
    ExportIsocam => [
        "Export for ISOCAM...",
        "Exportar para ISOCAM...",
        "Für ISOCAM exportieren...",
        "Exporter pour ISOCAM...",
        "Esporta per ISOCAM...",
        "ISOCAM 用に書き出す...",
        "导出给 ISOCAM...",
        "Exportar para o ISOCAM...",
    ],
    Fit => [
        "Fit",
        "Ajustar",
        "Einpassen",
        "Ajuster",
        "Adatta",
        "全体表示",
        "适应窗口",
        "Ajustar",
    ],
    Language => [
        "Language",
        "Idioma",
        "Sprache",
        "Langue",
        "Lingua",
        "言語",
        "语言",
        "Idioma",
    ],
    DropHint => [
        "Drop a PNG or PGM picture here or press Open picture",
        "Suelte aquí una imagen PNG o PGM o pulse Abrir imagen",
        "PNG- oder PGM-Bild hierher ziehen oder auf Bild öffnen klicken",
        "Déposez une image PNG ou PGM ici ou appuyez sur Ouvrir une image",
        "Trascina qui un'immagine PNG o PGM o premi Apri immagine",
        "PNG または PGM の画像をここにドロップするか「画像を開く」を押してください",
        "将 PNG 或 PGM 图像拖到此处，或点击“打开图像”",
        "Largue aqui uma imagem PNG ou PGM ou prima Abrir imagem",
    ],
    SearchTitle => [
        "Search for marks",
        "Búsqueda de marcas",
        "Suche nach Marken",
        "Recherche des repères",
        "Ricerca dei riferimenti",
        "マークの検出",
        "标记检测",
        "Procura de marcas",
    ],
    Threshold => [
        "Threshold (0 to 255 or auto)",
        "Umbral (0 a 255 o auto)",
        "Schwellwert (0 bis 255 oder auto)",
        "Seuil (0 à 255 ou auto)",
        "Soglia (da 0 a 255 o auto)",
        "しきい値 (0～255 または auto)",
        "阈值（0 到 255 或 auto）",
        "Limiar (0 a 255 ou auto)",
    ],
    MinDiameter => [
        "Smallest diameter (px)",
        "Diámetro mínimo (px)",
        "Kleinster Durchmesser (px)",
        "Diamètre minimal (px)",
        "Diametro minimo (px)",
        "最小直径 (px)",
        "最小直径 (px)",
        "Diâmetro mínimo (px)",
    ],
    MaxDiameter => [
        "Largest diameter (px)",
        "Diámetro máximo (px)",
        "Größter Durchmesser (px)",
        "Diamètre maximal (px)",
        "Diametro massimo (px)",
        "最大直径 (px)",
        "最大直径 (px)",
        "Diâmetro máximo (px)",
    ],
    MinFill => [
        "Fill of the circle (0 to 1)",
        "Relleno del círculo (0 a 1)",
        "Füllung des Kreises (0 bis 1)",
        "Remplissage du cercle (0 à 1)",
        "Riempimento del cerchio (da 0 a 1)",
        "円の充填率 (0～1)",
        "圆的填充率（0 到 1）",
        "Preenchimento do círculo (0 a 1)",
    ],
    MaxElongation => [
        "Largest elongation (1 is round)",
        "Elongación máxima (1 es redonda)",
        "Größte Streckung (1 ist rund)",
        "Allongement maximal (1 est rond)",
        "Allungamento massimo (1 è rotondo)",
        "最大の細長さ (1 で真円)",
        "最大伸长（1 为圆形）",
        "Alongamento máximo (1 é redondo)",
    ],
    LightMarks => [
        "Marks are light on a dark board",
        "Marcas claras sobre placa oscura",
        "Helle Marken auf dunkler Platine",
        "Repères clairs sur carte sombre",
        "Riferimenti chiari su scheda scura",
        "暗い基板上の明るいマーク",
        "深色板上的浅色标记",
        "Marcas claras sobre placa escura",
    ],
    Detect => [
        "Find marks",
        "Buscar marcas",
        "Marken suchen",
        "Chercher les repères",
        "Cerca i riferimenti",
        "マークを検出",
        "查找标记",
        "Procurar marcas",
    ],
    Marks => [
        "Marks",
        "Marcas",
        "Marken",
        "Repères",
        "Riferimenti",
        "マーク",
        "标记",
        "Marcas",
    ],
    NoMarks => [
        "No marks found",
        "No se encontró ninguna marca",
        "Keine Marken gefunden",
        "Aucun repère trouvé",
        "Nessun riferimento trovato",
        "マークが見つかりません",
        "未找到标记",
        "Nenhuma marca encontrada",
    ],
    ScaleTitle => [
        "Scale",
        "Escala",
        "Maßstab",
        "Échelle",
        "Scala",
        "スケール",
        "比例",
        "Escala",
    ],
    KnownMm => [
        "Size of the mark (mm)",
        "Tamaño de la marca (mm)",
        "Größe der Marke (mm)",
        "Taille du repère (mm)",
        "Dimensione del riferimento (mm)",
        "マークの大きさ (mm)",
        "标记大小 (mm)",
        "Tamanho da marca (mm)",
    ],
    UseSelected => [
        "Set the scale from the selected mark",
        "Fijar la escala con la marca elegida",
        "Maßstab aus der gewählten Marke bestimmen",
        "Fixer l'échelle avec le repère choisi",
        "Imposta la scala con il riferimento scelto",
        "選んだマークでスケールを設定",
        "用所选标记设定比例",
        "Definir a escala com a marca escolhida",
    ],
    ScaleLine => [
        "Scale: {0} mm per pixel",
        "Escala: {0} mm por píxel",
        "Maßstab: {0} mm pro Pixel",
        "Échelle : {0} mm par pixel",
        "Scala: {0} mm per pixel",
        "スケール: 1 ピクセルあたり {0} mm",
        "比例：每像素 {0} mm",
        "Escala: {0} mm por píxel",
    ],
    NoScale => [
        "No scale yet",
        "Aún no hay escala",
        "Noch kein Maßstab",
        "Pas encore d'échelle",
        "Ancora nessuna scala",
        "スケールは未設定です",
        "尚无比例",
        "Ainda sem escala",
    ],
    DistortionTitle => [
        "Lens distortion",
        "Distorsión del objetivo",
        "Verzeichnung des Objektivs",
        "Distorsion de l'objectif",
        "Distorsione dell'obiettivo",
        "レンズの歪み",
        "镜头畸变",
        "Distorção da objetiva",
    ],
    GridColumns => [
        "Columns of the grid",
        "Columnas de la rejilla",
        "Spalten des Rasters",
        "Colonnes de la grille",
        "Colonne della griglia",
        "グリッドの列数",
        "网格列数",
        "Colunas da grelha",
    ],
    GridRows => [
        "Rows of the grid",
        "Filas de la rejilla",
        "Zeilen des Rasters",
        "Lignes de la grille",
        "Righe della griglia",
        "グリッドの行数",
        "网格行数",
        "Linhas da grelha",
    ],
    FitDistortion => [
        "Fit the distortion from the grid",
        "Ajustar la distorsión con la rejilla",
        "Verzeichnung aus dem Raster bestimmen",
        "Ajuster la distorsion avec la grille",
        "Calcola la distorsione dalla griglia",
        "グリッドから歪みを求める",
        "用网格拟合畸变",
        "Ajustar a distorção com a grelha",
    ],
    ApplyDistortion => [
        "Correct the marks with it",
        "Corregir las marcas con ella",
        "Marken damit korrigieren",
        "Corriger les repères avec elle",
        "Correggi i riferimenti con essa",
        "これでマークを補正する",
        "用它校正标记",
        "Corrigir as marcas com ela",
    ],
    DistortionLine => [
        "k1 = {0}; the grid is off by {1} px before and {2} px after",
        "k1 = {0}; la rejilla se desvía {1} px antes y {2} px después",
        "k1 = {0}; das Raster weicht vorher um {1} px und nachher um {2} px ab",
        "k1 = {0} ; la grille s'écarte de {1} px avant et de {2} px après",
        "k1 = {0}; la griglia si discosta di {1} px prima e di {2} px dopo",
        "k1 = {0}、グリッドのずれ: 補正前 {1} px、補正後 {2} px",
        "k1 = {0}；网格偏差：校正前 {1} px，校正后 {2} px",
        "k1 = {0}; a grelha desvia-se {1} px antes e {2} px depois",
    ],
    AlignTitle => [
        "Alignment with the design",
        "Alineación con el diseño",
        "Ausrichtung am Entwurf",
        "Alignement sur le dessin",
        "Allineamento al disegno",
        "設計との位置合わせ",
        "与设计对齐",
        "Alinhamento com o desenho",
    ],
    DesignLine => [
        "Design: {0}, {1} points",
        "Diseño: {0}, {1} puntos",
        "Entwurf: {0}, {1} Punkte",
        "Dessin : {0}, {1} points",
        "Disegno: {0}, {1} punti",
        "設計: {0}、{1} 点",
        "设计：{0}，{1} 个点",
        "Desenho: {0}, {1} pontos",
    ],
    NoDesign => [
        "No design loaded",
        "Ningún diseño cargado",
        "Kein Entwurf geladen",
        "Aucun dessin chargé",
        "Nessun disegno caricato",
        "設計は読み込まれていません",
        "未载入设计",
        "Nenhum desenho carregado",
    ],
    AllowScale => [
        "Allow a change of size",
        "Permitir un cambio de tamaño",
        "Größenänderung zulassen",
        "Autoriser un changement de taille",
        "Consenti un cambio di dimensione",
        "サイズの変更を許可",
        "允许改变尺寸",
        "Permitir uma alteração de tamanho",
    ],
    MaxRms => [
        "Largest residual (mm)",
        "Residuo máximo (mm)",
        "Größter Restfehler (mm)",
        "Résidu maximal (mm)",
        "Residuo massimo (mm)",
        "最大残差 (mm)",
        "最大残差 (mm)",
        "Resíduo máximo (mm)",
    ],
    MaxStretch => [
        "Largest change of size (0.01 = 1 %)",
        "Cambio de tamaño máximo (0.01 = 1 %)",
        "Größte Größenänderung (0.01 = 1 %)",
        "Changement de taille maximal (0.01 = 1 %)",
        "Cambio di dimensione massimo (0.01 = 1 %)",
        "サイズ変化の上限 (0.01 = 1 %)",
        "最大尺寸变化（0.01 = 1 %）",
        "Alteração de tamanho máxima (0.01 = 1 %)",
    ],
    Align => [
        "Align",
        "Alinear",
        "Ausrichten",
        "Aligner",
        "Allinea",
        "位置合わせ",
        "对齐",
        "Alinhar",
    ],
    ShiftLine => [
        "Shift: ({0}, {1}) mm",
        "Desplazamiento: ({0}, {1}) mm",
        "Verschiebung: ({0}, {1}) mm",
        "Décalage : ({0}, {1}) mm",
        "Spostamento: ({0}, {1}) mm",
        "シフト: ({0}, {1}) mm",
        "平移：({0}, {1}) mm",
        "Deslocamento: ({0}, {1}) mm",
    ],
    TurnLine => [
        "Turn: {0} degrees",
        "Giro: {0} grados",
        "Drehung: {0} Grad",
        "Rotation : {0} degrés",
        "Rotazione: {0} gradi",
        "回転: {0} 度",
        "旋转：{0} 度",
        "Rotação: {0} graus",
    ],
    SizeLine => [
        "Size: {0}",
        "Tamaño: {0}",
        "Größe: {0}",
        "Taille : {0}",
        "Dimensione: {0}",
        "サイズ: {0}",
        "尺寸：{0}",
        "Tamanho: {0}",
    ],
    ResidualLine => [
        "Residual: {0} mm",
        "Residuo: {0} mm",
        "Restfehler: {0} mm",
        "Résidu : {0} mm",
        "Residuo: {0} mm",
        "残差: {0} mm",
        "残差：{0} mm",
        "Resíduo: {0} mm",
    ],
    Accepted => [
        "Accepted: the marks fit the design",
        "Aceptada: las marcas encajan con el diseño",
        "Angenommen: die Marken passen zum Entwurf",
        "Acceptée : les repères correspondent au dessin",
        "Accettato: i riferimenti corrispondono al disegno",
        "合格: マークは設計に合っています",
        "通过：标记与设计吻合",
        "Aceite: as marcas encaixam no desenho",
    ],
    RefusedRms => [
        "Refused: the marks fit the design to {0} mm, more than the {1} mm allowed; one of them is probably not the one it should be",
        "Rechazada: las marcas encajan con el diseño con {0} mm, más de los {1} mm permitidos; probablemente una no es la que debería",
        "Abgelehnt: die Marken passen nur auf {0} mm zum Entwurf, mehr als die erlaubten {1} mm; wahrscheinlich ist eine nicht die richtige",
        "Refusée : les repères s'accordent au dessin à {0} mm, plus que les {1} mm permis ; l'un d'eux n'est probablement pas le bon",
        "Rifiutato: i riferimenti corrispondono al disegno a {0} mm, più dei {1} mm ammessi; probabilmente uno non è quello giusto",
        "不合格: マークと設計のずれは {0} mm で、許容の {1} mm を超えています。どれかが違うマークの可能性があります",
        "未通过：标记与设计的偏差为 {0} mm，超过允许的 {1} mm；其中某个标记可能不对",
        "Recusado: as marcas ajustam-se ao desenho a {0} mm, mais do que os {1} mm permitidos; provavelmente uma delas não é a que devia ser",
    ],
    RefusedStretch => [
        "Refused: the board is {0} % bigger or smaller than its design, more than the {1} % allowed",
        "Rechazada: la placa es un {0} % mayor o menor que su diseño, más del {1} % permitido",
        "Abgelehnt: die Platine ist {0} % größer oder kleiner als ihr Entwurf, mehr als die erlaubten {1} %",
        "Refusée : la carte est plus grande ou plus petite de {0} % que son dessin, plus que les {1} % permis",
        "Rifiutato: la scheda è più grande o più piccola del {0} % rispetto al disegno, oltre l'{1} % ammesso",
        "不合格: 基板が設計より {0} % 大きいか小さく、許容の {1} % を超えています",
        "未通过：电路板比设计大或小 {0} %，超过允许的 {1} %",
        "Recusado: a placa é {0} % maior ou menor do que o seu desenho, mais do que os {1} % permitidos",
    ],
    IsocamTitle => [
        "Options for ISOCAM",
        "Opciones para ISOCAM",
        "Optionen für ISOCAM",
        "Options pour ISOCAM",
        "Opzioni per ISOCAM",
        "ISOCAM 用オプション",
        "ISOCAM 选项",
        "Opções para o ISOCAM",
    ],
    Results => [
        "Results",
        "Resultados",
        "Ergebnisse",
        "Résultats",
        "Risultati",
        "結果",
        "结果",
        "Resultados",
    ],
    ZoomHint => [
        "Wheel: zoom - drag: move - double click: fit",
        "Rueda: zoom - arrastrar: mover - doble clic: ajustar",
        "Rad: Zoom - Ziehen: verschieben - Doppelklick: einpassen",
        "Molette : zoom - glisser : déplacer - double clic : ajuster",
        "Rotella: zoom - trascina: sposta - doppio clic: adatta",
        "ホイール: ズーム - ドラッグ: 移動 - ダブルクリック: 全体表示",
        "滚轮：缩放 - 拖动：移动 - 双击：适应窗口",
        "Roda: zoom - arrastar: mover - duplo clique: ajustar",
    ],
    OpenImageTitle => [
        "Open a picture",
        "Abrir una imagen",
        "Ein Bild öffnen",
        "Ouvrir une image",
        "Apri un'immagine",
        "画像を開く",
        "打开图像",
        "Abrir uma imagem",
    ],
    OpenDesignTitle => [
        "Load the design",
        "Cargar el diseño",
        "Den Entwurf laden",
        "Charger le dessin",
        "Carica il disegno",
        "設計を読み込む",
        "载入设计",
        "Carregar o desenho",
    ],
    ExportTitle => [
        "Export the options for ISOCAM",
        "Exportar las opciones para ISOCAM",
        "Optionen für ISOCAM exportieren",
        "Exporter les options pour ISOCAM",
        "Esporta le opzioni per ISOCAM",
        "ISOCAM 用オプションを書き出す",
        "导出 ISOCAM 选项",
        "Exportar as opções para o ISOCAM",
    ],
    Export => [
        "Export",
        "Exportar",
        "Exportieren",
        "Exporter",
        "Esporta",
        "書き出す",
        "导出",
        "Exportar",
    ],
    Up => [
        "Up",
        "Subir",
        "Hinauf",
        "Monter",
        "Su",
        "上へ",
        "上一级",
        "Subir",
    ],
    AllFiles => [
        "All files",
        "Todos los ficheros",
        "Alle Dateien",
        "Tous les fichiers",
        "Tutti i file",
        "すべてのファイル",
        "所有文件",
        "Todos os ficheiros",
    ],
    Cancel => [
        "Cancel",
        "Cancelar",
        "Abbrechen",
        "Annuler",
        "Annulla",
        "キャンセル",
        "取消",
        "Cancelar",
    ],
    EmptyFolder => [
        "Nothing to open in this folder",
        "Nada que abrir en esta carpeta",
        "Nichts zum Öffnen in diesem Ordner",
        "Rien à ouvrir dans ce dossier",
        "Niente da aprire in questa cartella",
        "このフォルダーに開けるものはありません",
        "此文件夹中没有可打开的内容",
        "Nada para abrir nesta pasta",
    ],
    FileName => [
        "File name",
        "Nombre del fichero",
        "Dateiname",
        "Nom du fichier",
        "Nome del file",
        "ファイル名",
        "文件名",
        "Nome do ficheiro",
    ],
    Ready => [
        "Open a picture to begin",
        "Abra una imagen para empezar",
        "Bild öffnen, um zu beginnen",
        "Ouvrez une image pour commencer",
        "Apri un'immagine per iniziare",
        "画像を開いて始めてください",
        "请先打开图像",
        "Abra uma imagem para começar",
    ],
    Opened => [
        "{0}: {1} x {2} pixels",
        "{0}: {1} x {2} píxeles",
        "{0}: {1} x {2} Pixel",
        "{0} : {1} x {2} pixels",
        "{0}: {1} x {2} pixel",
        "{0}: {1} x {2} ピクセル",
        "{0}：{1} x {2} 像素",
        "{0}: {1} x {2} píxeis",
    ],
    Found => [
        "{0} marks found",
        "{0} marcas encontradas",
        "{0} Marken gefunden",
        "{0} repères trouvés",
        "{0} riferimenti trovati",
        "マークを {0} 個検出しました",
        "找到 {0} 个标记",
        "{0} marcas encontradas",
    ],
    ScaleSet => [
        "Scale set: {0} mm per pixel",
        "Escala fijada: {0} mm por píxel",
        "Maßstab gesetzt: {0} mm pro Pixel",
        "Échelle fixée : {0} mm par pixel",
        "Scala impostata: {0} mm per pixel",
        "スケールを設定しました: 1 ピクセルあたり {0} mm",
        "已设定比例：每像素 {0} mm",
        "Escala definida: {0} mm por píxel",
    ],
    DistortionSet => [
        "Distortion fitted: k1 = {0}",
        "Distorsión ajustada: k1 = {0}",
        "Verzeichnung bestimmt: k1 = {0}",
        "Distorsion ajustée : k1 = {0}",
        "Distorsione calcolata: k1 = {0}",
        "歪みを求めました: k1 = {0}",
        "已拟合畸变：k1 = {0}",
        "Distorção ajustada: k1 = {0}",
    ],
    DesignLoaded => [
        "{0}: {1} points of the design",
        "{0}: {1} puntos del diseño",
        "{0}: {1} Punkte des Entwurfs",
        "{0} : {1} points du dessin",
        "{0}: {1} punti del disegno",
        "{0}: 設計の {1} 点",
        "{0}：设计的 {1} 个点",
        "{0}: {1} pontos do desenho",
    ],
    Aligned => [
        "Aligned: shift ({0}, {1}) mm, turn {2} degrees",
        "Alineado: desplazamiento ({0}, {1}) mm, giro {2} grados",
        "Ausgerichtet: Verschiebung ({0}, {1}) mm, Drehung {2} Grad",
        "Aligné : décalage ({0}, {1}) mm, rotation {2} degrés",
        "Allineato: spostamento ({0}, {1}) mm, rotazione {2} gradi",
        "位置合わせ完了: シフト ({0}, {1}) mm、回転 {2} 度",
        "已对齐：平移 ({0}, {1}) mm，旋转 {2} 度",
        "Alinhado: deslocamento ({0}, {1}) mm, rotação {2} graus",
    ],
    Exported => [
        "Written {0}",
        "Escrito {0}",
        "Geschrieben: {0}",
        "Écrit : {0}",
        "Scritto {0}",
        "書き出しました: {0}",
        "已写入 {0}",
        "Escrito {0}",
    ],
    Failed => [
        "Error: {0}",
        "Error: {0}",
        "Fehler: {0}",
        "Erreur : {0}",
        "Errore: {0}",
        "エラー: {0}",
        "错误：{0}",
        "Erro: {0}",
    ],
    BadValue => [
        "{0}: that value is not valid",
        "{0}: ese valor no es válido",
        "{0}: dieser Wert ist ungültig",
        "{0} : cette valeur n'est pas valide",
        "{0}: questo valore non è valido",
        "{0}: その値は無効です",
        "{0}：该值无效",
        "{0}: esse valor não é válido",
    ],
    NeedImage => [
        "Open a picture first",
        "Abra primero una imagen",
        "Zuerst ein Bild öffnen",
        "Ouvrez d'abord une image",
        "Apri prima un'immagine",
        "先に画像を開いてください",
        "请先打开图像",
        "Abra primeiro uma imagem",
    ],
    NeedMark => [
        "Select a mark in the list first",
        "Elija primero una marca de la lista",
        "Zuerst eine Marke in der Liste wählen",
        "Choisissez d'abord un repère dans la liste",
        "Scegli prima un riferimento dall'elenco",
        "先にリストからマークを選んでください",
        "请先在列表中选择标记",
        "Escolha primeiro uma marca da lista",
    ],
    NeedScale => [
        "Set the scale first",
        "Fije primero la escala",
        "Zuerst den Maßstab setzen",
        "Fixez d'abord l'échelle",
        "Imposta prima la scala",
        "先にスケールを設定してください",
        "请先设定比例",
        "Defina primeiro a escala",
    ],
    NeedDesign => [
        "Load the design first",
        "Cargue primero el diseño",
        "Zuerst den Entwurf laden",
        "Chargez d'abord le dessin",
        "Carica prima il disegno",
        "先に設計を読み込んでください",
        "请先载入设计",
        "Carregue primeiro o desenho",
    ],
    NeedAccepted => [
        "There is no accepted alignment to export",
        "No hay una alineación aceptada que exportar",
        "Es gibt keine angenommene Ausrichtung zum Exportieren",
        "Il n'y a aucun alignement accepté à exporter",
        "Non c'è un allineamento accettato da esportare",
        "書き出せる合格済みの位置合わせがありません",
        "没有可导出的通过的对齐结果",
        "Não há um alinhamento aceite para exportar",
    ],
    GridCount => [
        "The grid needs {0} marks and {1} were found",
        "La rejilla necesita {0} marcas y se encontraron {1}",
        "Das Raster braucht {0} Marken, gefunden wurden {1}",
        "La grille demande {0} repères et {1} ont été trouvés",
        "La griglia richiede {0} riferimenti e ne sono stati trovati {1}",
        "グリッドには {0} 個のマークが必要ですが {1} 個でした",
        "网格需要 {0} 个标记，但找到了 {1} 个",
        "A grelha precisa de {0} marcas e foram encontradas {1}",
    ],
    TooLarge => [
        "{0}: the file is larger than {1} MiB",
        "{0}: el fichero supera {1} MiB",
        "{0}: die Datei ist größer als {1} MiB",
        "{0} : le fichier dépasse {1} MiB",
        "{0}: il file supera {1} MiB",
        "{0}: ファイルが {1} MiB を超えています",
        "{0}：文件超过 {1} MiB",
        "{0}: o ficheiro excede {1} MiB",
    ],
    SameFile => [
        "Choose another name: that file was opened by the program",
        "Elija otro nombre: ese fichero lo abrió el programa",
        "Anderen Namen wählen: diese Datei wurde vom Programm geöffnet",
        "Choisissez un autre nom : ce fichier a été ouvert par le programme",
        "Scegli un altro nome: quel file è stato aperto dal programma",
        "別の名前を選んでください: そのファイルはプログラムが開いたものです",
        "请换一个名称：该文件已被程序打开",
        "Escolha outro nome: esse ficheiro foi aberto pelo programa",
    ],
    ConfigProblem => [
        "The configuration file was not used: {0}",
        "No se usó el fichero de configuración: {0}",
        "Die Konfigurationsdatei wurde nicht verwendet: {0}",
        "Le fichier de configuration n'a pas été utilisé : {0}",
        "Il file di configurazione non è stato usato: {0}",
        "設定ファイルは使われませんでした: {0}",
        "未使用配置文件：{0}",
        "O ficheiro de configuração não foi usado: {0}",
    ],
}

impl Msg {
    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }

    /// The text in a language with the values put in its places `{0}`, `{1}`...
    pub fn with<S: AsRef<str>>(self, lang: Lang, values: &[S]) -> String {
        fill(self.tr(lang), values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..6)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Language.tr(Lang::Pt), "Idioma");
        assert_eq!(Msg::Up.tr(Lang::En), "Up");
    }

    #[test]
    fn the_texts_of_portugal_use_its_spelling() {
        assert_eq!(Msg::FileName.tr(Lang::Pt), "Nome do ficheiro");
        assert!(Msg::GridRows.tr(Lang::Pt).contains("grelha"));
        assert!(Msg::AllFiles.tr(Lang::Pt).contains("ficheiros"));
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_configuration_names_a_language_or_leaves_it_to_the_system() {
        assert_eq!(Lang::from_setting("de"), Lang::De);
        assert_eq!(Lang::from_setting("zh"), Lang::Zh);
        assert_eq!(Lang::from_setting("xx"), Lang::En);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
        assert_eq!(
            Msg::ScaleLine.with(Lang::En, &["0.05"]),
            "Scale: 0.05 mm per pixel"
        );
    }
}
