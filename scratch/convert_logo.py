from PIL import Image, ImageDraw, ImageChops

png_path = r"c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\homepage\public\logo.png"
ico_path = r"c:\Users\wjdwl\.gemini\antigravity\scratch\14-Keysor\keysor.ico"

print(f"Loading image from {png_path}...")
img = Image.open(png_path).convert("RGBA")
width, height = img.size
print(f"Original image size: {width}x{height}")

# Create rounded corner mask (iOS/macOS style radius, ~28% of the image size)
mask = Image.new("L", (width, height), 0)
draw = ImageDraw.Draw(mask)
radius = int(width * 0.28) # 28% of width (e.g., 143 for 512px)

print(f"Applying rounded corner mask with radius: {radius}px...")
draw.rounded_rectangle((0, 0, width, height), radius=radius, fill=255)

# Blend with original alpha channel if it exists
if "A" in img.getbands():
    orig_alpha = img.getchannel("A")
    mask = ImageChops.multiply(mask, orig_alpha)

img.putalpha(mask)

# Standard Windows icon sizes
icon_sizes = [(16, 16), (32, 32), (48, 48), (256, 256)]

print(f"Saving to {ico_path} with sizes {icon_sizes}...")
img.save(ico_path, sizes=icon_sizes)
print("Conversion with rounded corners complete!")
