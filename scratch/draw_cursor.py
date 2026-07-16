import os
from PIL import Image, ImageDraw

def draw_cursor(filename, line3_coords):
    scale = 10
    img = Image.new('RGBA', (320, 320), (255, 255, 255, 0))
    draw = ImageDraw.Draw(img)
    
    p1 = (16 * scale, 16 * scale)
    p2 = (19 * scale, 29 * scale)
    p3 = (30 * scale, 27 * scale)
    p4 = (line3_coords[0] * scale, line3_coords[1] * scale)
    p5 = (line3_coords[2] * scale, line3_coords[3] * scale)
    
    border_width = int(4 * scale)
    draw.line([p1, p2], fill=(0, 0, 0, 255), width=border_width, joint='round')
    draw.line([p1, p3], fill=(0, 0, 0, 255), width=border_width, joint='round')
    draw.line([p4, p5], fill=(0, 0, 0, 255), width=border_width, joint='round')
    
    line_width = int(2.5 * scale)
    
    def draw_grad_line(start, end):
        segments = 30
        for i in range(segments):
            t1 = i / segments
            t2 = (i + 1) / segments
            pt1 = (start[0] + (end[0] - start[0]) * t1, start[1] + (end[1] - start[1]) * t1)
            pt2 = (start[0] + (end[0] - start[0]) * t2, start[1] + (end[1] - start[1]) * t2)
            
            grad_vec = (p3[0] - p1[0], p3[1] - p1[1])
            grad_len2 = grad_vec[0]**2 + grad_vec[1]**2
            
            mid_pt = ((pt1[0] + pt2[0])/2, (pt1[1] + pt2[1])/2)
            mid_vec = (mid_pt[0] - p1[0], mid_pt[1] - p1[1])
            
            if grad_len2 > 0:
                t = (mid_vec[0]*grad_vec[0] + mid_vec[1]*grad_vec[1]) / grad_len2
            else:
                t = 0
            t = max(0.0, min(1.0, t))
            
            # Neon Green (47, 255, 173) to Dark Green (0, 77, 32)
            r = int(47 + (0 - 47) * t)
            g = int(255 + (77 - 255) * t)
            b = int(173 + (32 - 173) * t)
            draw.line([pt1, pt2], fill=(r, g, b, 255), width=line_width, joint='round')

    draw_grad_line(p1, p2)
    draw_grad_line(p1, p3)
    draw_grad_line(p4, p5)
    
    # Save directly in scratch folder
    img.save(filename)

os.makedirs('scratch', exist_ok=True)
draw_cursor('scratch/cursor_current.png', (18, 25, 25, 14))
draw_cursor('scratch/cursor_proposed.png', (19, 29, 30, 27))
print("Images saved successfully.")
