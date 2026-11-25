import sys
from PIL import Image, ImageDraw

out, tray = sys.argv[1], sys.argv[2]
S = 1024
im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(im)
d.rounded_rectangle((100, 100, 924, 924), 185, fill="#1f2a44")
d.rounded_rectangle((230, 270, 794, 640), 44, outline="#e9eefb", width=34)
d.ellipse((430, 373, 594, 537), fill="#ff5a4f")
for y, w in ((710, 564), (776, 380)):
    d.rounded_rectangle((230, y, 230 + w, y + 30), 15, fill="#8fa3d4")
im.save(out)
t = Image.new("RGBA", (176, 176), (0, 0, 0, 0))
d = ImageDraw.Draw(t)
d.rounded_rectangle((16, 32, 160, 144), 22, outline="black", width=14)
d.ellipse((62, 62, 114, 114), fill="black")
t.resize((44, 44), Image.LANCZOS).save(tray)
