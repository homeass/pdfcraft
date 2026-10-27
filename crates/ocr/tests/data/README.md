# Korean OCR test data

`fixture-ko.png`: a white A4 page (1654 × 2339, 200 dpi) with three lines of text at 64 px
(48 pt):

1. `한국어 스캔 테스트 검색 가능한 스캔`
2. `학교안심 나눔고딕 문서 인식 확인`
3. `2026년 10월 27일 PdfCraft OCR`

It was generated for these tests with PIL drawing the openly licensed NanumGothic face
(SIL OFL-1.1) onto the page. The font drew the picture only: nothing from it is embedded,
bundled or shipped, and the image itself is contributor-original.

Regenerate with (any machine with Python 3, Pillow and the font):

```python
from PIL import Image, ImageDraw, ImageFont
font = ImageFont.truetype("NanumGothic-Regular.ttf", 64)
img = Image.new("RGB", (1654, 2339), "white")
d = ImageDraw.Draw(img)
for i, line in enumerate([
    "한국어 스캔 테스트 검색 가능한 스캔",
    "학교안심 나눔고딕 문서 인식 확인",
    "2026년 10월 27일 PdfCraft OCR",
]):
    d.text((160, 280 + i * 162), line, font=font, fill="black")
img.save("fixture-ko.png")
```

The recognition end-to-end test (`crates/ocr/src/paddle.rs`) reads it when the Korean models
are installed (`cargo xtask models`) and skips otherwise. Single-vowel confusions
(`나눔고딕` → `나눔고닥`) are known model behaviour, so the test expects 14 of the 15 distinct
words exactly plus the two lines the model reads cleanly, and fails if recognition collapses.
