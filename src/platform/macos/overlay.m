#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

@interface KeysorCursorView : NSView
@property (nonatomic, assign) int cursorState;
@end

@implementation KeysorCursorView

- (void)drawRect:(NSRect)dirtyRect {
    [super drawRect:dirtyRect];
    CGContextRef ctx = [[NSGraphicsContext currentContext] CGContext];
    if (!ctx) return;

    CGContextClearRect(ctx, dirtyRect);

    // 1. 검정 고대비 외곽 테두리 (4.0pt, 라운드 캡/조인)
    CGContextSaveGState(ctx);
    CGContextSetLineWidth(ctx, 4.0);
    CGContextSetLineCap(ctx, kCGLineCapRound);
    CGContextSetLineJoin(ctx, kCGLineJoinRound);
    CGContextSetRGBStrokeColor(ctx, 0.0, 0.0, 0.0, 1.0);

    // 라인 1: (16, 16) -> (19, 29)
    CGContextMoveToPoint(ctx, 16.0, 36.0 - 16.0);
    CGContextAddLineToPoint(ctx, 19.0, 36.0 - 29.0);

    // 라인 2: (16, 16) -> (30, 27)
    CGContextMoveToPoint(ctx, 16.0, 36.0 - 16.0);
    CGContextAddLineToPoint(ctx, 30.0, 36.0 - 27.0);

    // 라인 3: (18, 25) -> (25, 14)
    CGContextMoveToPoint(ctx, 18.0, 36.0 - 25.0);
    CGContextAddLineToPoint(ctx, 25.0, 36.0 - 14.0);

    CGContextStrokePath(ctx);
    CGContextRestoreGState(ctx);

    // 2. 키서 정품 라이너 그라디언트 스트로크 준비 (2.5pt)
    CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();
    CGFloat components[8];

    if (self.cursorState == 1) {
        // 좌클릭/스페이스: Neon Orange (0xFFFF4500) -> Red (0xFFFF0000)
        components[0] = 1.0;   components[1] = 0.271; components[2] = 0.0;   components[3] = 1.0;
        components[4] = 1.0;   components[5] = 0.0;   components[6] = 0.0;   components[7] = 1.0;
    } else if (self.cursorState == 2) {
        // 우클릭/G키: Yellow (0xFFFFFF00) -> Gold (0xFFFFAA00)
        components[0] = 1.0;   components[1] = 1.0;   components[2] = 0.0;   components[3] = 1.0;
        components[4] = 1.0;   components[5] = 0.667; components[6] = 0.0;   components[7] = 1.0;
    } else if (self.cursorState == 3) {
        // 스크롤: Neon Cyan (0xFF00E5FF) -> Blue (0xFF0055FF)
        components[0] = 0.0;   components[1] = 0.898; components[2] = 1.0;   components[3] = 1.0;
        components[4] = 0.0;   components[5] = 0.333; components[6] = 1.0;   components[7] = 1.0;
    } else if (self.cursorState == 4) {
        // 드래그: Neon Orange (0xFFFF4500) -> Pink-Red (0xFFFF007F)
        components[0] = 1.0;   components[1] = 0.271; components[2] = 0.0;   components[3] = 1.0;
        components[4] = 1.0;   components[5] = 0.0;   components[6] = 0.498; components[7] = 1.0;
    } else {
        // 키서 정식 시그니처: Neon Green (0xFF2FFFAD) -> Dark Forest Green (0xFF004D20)
        components[0] = 0.184; components[1] = 1.0;   components[2] = 0.678; components[3] = 1.0;
        components[4] = 0.0;   components[5] = 0.302; components[6] = 0.125; components[7] = 1.0;
    }

    CGGradientRef gradient = CGGradientCreateWithColorComponents(colorSpace, components, NULL, 2);

    CGContextSaveGState(ctx);
    CGContextSetLineWidth(ctx, 2.5);
    CGContextSetLineCap(ctx, kCGLineCapRound);
    CGContextSetLineJoin(ctx, kCGLineJoinRound);

    CGContextMoveToPoint(ctx, 16.0, 36.0 - 16.0);
    CGContextAddLineToPoint(ctx, 19.0, 36.0 - 29.0);

    CGContextMoveToPoint(ctx, 16.0, 36.0 - 16.0);
    CGContextAddLineToPoint(ctx, 30.0, 36.0 - 27.0);

    CGContextMoveToPoint(ctx, 18.0, 36.0 - 25.0);
    CGContextAddLineToPoint(ctx, 25.0, 36.0 - 14.0);

    CGContextReplacePathWithStrokedPath(ctx);
    CGContextClip(ctx);

    CGPoint startPt = CGPointMake(16.0, 36.0 - 16.0);
    CGPoint endPt   = CGPointMake(30.0, 36.0 - 27.0);

    CGContextDrawLinearGradient(ctx, gradient, startPt, endPt, 0);
    CGContextRestoreGState(ctx);

    CGGradientRelease(gradient);
    CGColorSpaceRelease(colorSpace);
}

@end

static NSWindow *g_overlayWindow = nil;
static KeysorCursorView *g_cursorView = nil;

void keysor_macos_ui_init(void) {
    if (g_overlayWindow != nil) return;

    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];

        NSRect frame = NSMakeRect(0, 0, 36, 36);
        g_overlayWindow = [[NSWindow alloc] initWithContentRect:frame
                                                      styleMask:NSWindowStyleMaskBorderless
                                                        backing:NSBackingStoreBuffered
                                                          defer:NO];

        [g_overlayWindow setBackgroundColor:[NSColor clearColor]];
        [g_overlayWindow setOpaque:NO];
        [g_overlayWindow setHasShadow:NO];
        [g_overlayWindow setLevel:NSStatusWindowLevel]; // 25
        [g_overlayWindow setCollectionBehavior:NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorStationary];
        [g_overlayWindow setIgnoresMouseEvents:YES];

        g_cursorView = [[KeysorCursorView alloc] initWithFrame:frame];
        [g_overlayWindow setContentView:g_cursorView];
        NSLog(@"[Keysor Native ObjC UI] Keysor Original Dedicated Cursor Overlay Window initialized: %@", g_overlayWindow);
    });
}

void keysor_macos_ui_show(bool visible) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_overlayWindow == nil) return;
        if (visible) {
            [g_overlayWindow setAlphaValue:0.95];
            [g_overlayWindow orderFrontRegardless];
            [g_overlayWindow display];
        } else {
            [g_overlayWindow orderOut:nil];
        }
    });
}

void keysor_macos_ui_update_pos(double x, double y) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_overlayWindow == nil) return;
        NSRect displayBounds = [[NSScreen mainScreen] frame];
        double screen_h = displayBounds.size.height;
        // 커서 핫스팟 (16, 16) 정밀 동기화
        NSPoint origin = NSMakePoint(x - 16.0, screen_h - y - 20.0);
        [g_overlayWindow setFrameOrigin:origin];
    });
}

void keysor_macos_ui_set_click_motion(int clickType) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_cursorView == nil) return;
        g_cursorView.cursorState = clickType;
        [g_cursorView setNeedsDisplay:YES];
    });
}
