# Fonts

| File | Font | Licence | Used for |
|---|---|---|---|
| `DejaVuSans*.ttf` | DejaVu Sans (subset) | `LICENSE-DejaVu.txt` | browser UI and canvas; Arabic everywhere |
| `NotoSansDevanagari-*.ttf` | Noto Sans Devanagari | `LICENSE-Noto.txt` (OFL 1.1) | Hindi and other Devanagari text |
| `NotoSansThai-*.ttf` | Noto Sans Thai | `LICENSE-Noto.txt` (OFL 1.1) | Thai |
| `NotoSansArabic-Urdu-*.ttf` | Noto Sans Arabic, subset | `LICENSE-Noto.txt` (OFL 1.1) | Arabic-script letters DejaVu lacks (Urdu ہ ے ۔ …) |
| `NotoSansCJK-sub.otf` | Noto Sans CJK SC, subset | `LICENSE-Noto.txt` (OFL 1.1) | Chinese and Japanese |

`NotoSansCJK-sub.otf` keeps the everyday characters only (about 9,900): GB 2312 (simplified
Chinese), Big5 level 1 (traditional Chinese), JIS X 0208 level 1 (Japanese kanji), kana, CJK
punctuation and full-width forms. Made with fontTools from `NotoSansCJK-Regular.ttc` (face 2, SC):

    pyftsubset NotoSansCJK-Regular.ttc --font-number=2 --text-file=cjk-chars.txt \
      --output-file=NotoSansCJK-sub.otf --layout-features='*' --no-hinting

The desktop apps build it in; the browser version downloads it from `fonts/` when a drawing
first shows Chinese or Japanese text.

`NotoSansArabic-Urdu-*.ttf` keeps only the Arabic-block and presentation-form code points of
Noto Sans Arabic (Fontsource 5.3.0 build) that DejaVu Sans does not have, with no layout tables:
the canvas draws Arabic as presentation forms (see `src/rtl.rs`).

`cjk-chars.txt` also gets every CJK character used in `locales/` so the UI never shows boxes.
