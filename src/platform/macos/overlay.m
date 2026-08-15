#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <Carbon/Carbon.h>

// Rust ↔ C FFI 콜백 함수
extern void keysor_macos_on_lang_toggle(void);
extern double keysor_macos_on_speed_change(double delta);
extern double keysor_macos_get_base_speed(void);
extern bool keysor_macos_check_autostart_status(void);
extern bool keysor_macos_on_autostart_toggle(void);
extern bool keysor_macos_check_magnet_status(void);
extern bool keysor_macos_on_magnet_toggle(void);
extern bool keysor_macos_check_grid_status(void);
extern bool keysor_macos_on_grid_toggle(void);
extern bool keysor_macos_check_is_snapped(void);

@interface KeysorCursorView : NSView
@property (nonatomic, assign) int cursorState;
@end

@implementation KeysorCursorView

- (void)drawRect:(NSRect)dirtyRect {
    [super drawRect:dirtyRect];
    CGContextRef ctx = [[NSGraphicsContext currentContext] CGContext];
    if (!ctx) return;

    CGContextClearRect(ctx, dirtyRect);

    // Retina / 고해상도 디스플레이 선명도 안티앨리어싱 및 서브픽셀 정밀도 보정 (Item 3)
    CGContextSetShouldAntialias(ctx, true);
    CGContextSetAllowsAntialiasing(ctx, true);
    CGContextSetInterpolationQuality(ctx, kCGInterpolationHigh);

    // 1. 눌리는 감촉 (작아졌다가 커지는 누르는 듯한 모션: scale 0.70으로 더 확실하게 축소)
    CGFloat scale = (self.cursorState >= 1 && self.cursorState <= 4) ? 0.70 : 1.0;

    CGContextSaveGState(ctx);
    if (scale != 1.0) {
        CGContextTranslateCTM(ctx, 16.0, 36.0 - 16.0);
        CGContextScaleCTM(ctx, scale, scale);
        CGContextTranslateCTM(ctx, -16.0, -(36.0 - 16.0));
    }

    // 1-1. 검정 고대비 외곽 테두리 (4.0pt, 라운드 캡/조인)
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
    } else if (self.cursorState == 5) {
        // 자석 모드 흡착 (Magnet Snapped): Hot Pink (0xFFFF1493) -> Neon Magenta (0xFFFF0066)
        components[0] = 1.0;   components[1] = 0.078; components[2] = 0.576; components[3] = 1.0;
        components[4] = 1.0;   components[5] = 0.0;   components[6] = 0.392; components[7] = 1.0;
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

    CGContextRestoreGState(ctx);
}

@end

// =========================================================================
// Keysor macOS HUD 설명 및 설정 팝업 윈도우 View
// =========================================================================
@interface KeysorHudView : NSView
@property (nonatomic, assign) BOOL langIsEnglish;
@property (nonatomic, assign) double baseSpeed;
@property (nonatomic, assign) BOOL autoStartEnabled;
@property (nonatomic, assign) BOOL magnetModeEnabled;
@property (nonatomic, assign) BOOL gridModeEnabled;
@property (nonatomic, assign) BOOL showAllDetails;
@property (nonatomic, assign) NSPoint dragStartPoint;
@property (nonatomic, assign) int pressedButtonTag;
@property (nonatomic, assign) CGFloat animGridProgress;
@property (nonatomic, assign) CGFloat animMagnetProgress;
@property (nonatomic, assign) CGFloat animAutoStartProgress;
@property (nonatomic, retain) NSTimer *animTimer;
- (void)startToggleAnimTimer;
@end

@implementation KeysorHudView

// =========================================================================
// Color Palette Provider (중앙집중식 색상 유틸리티 함수)
// =========================================================================
static inline NSColor* keysorNeonGreenColor(CGFloat alpha) {
    return [NSColor colorWithCalibratedRed:0.184 green:1.0 blue:0.678 alpha:alpha];
}

static inline NSColor* keysorDarkGreenShadowColor(CGFloat alpha) {
    return [NSColor colorWithCalibratedRed:0.0 green:0.302 blue:0.125 alpha:alpha];
}

static inline NSColor* keysorPanelBgColor(CGFloat alpha) {
    return [NSColor colorWithCalibratedRed:0.07 green:0.07 blue:0.06 alpha:alpha];
}

static inline NSColor* keysorButtonBgColor(void) {
    return [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0];
}

static inline NSColor* keysorButtonBorderColor(void) {
    return [NSColor colorWithCalibratedRed:0.24 green:0.25 blue:0.25 alpha:1.0];
}

static void drawKeyCap(NSRect rect, NSString *key, NSString *desc, int category) {
    NSColor *bgColor = [NSColor colorWithCalibratedRed:0.125 green:0.141 blue:0.141 alpha:1.0];
    NSColor *borderColor = nil;
    NSColor *textColor = nil;

    switch (category) {
        case 1: borderColor = [NSColor colorWithCalibratedRed:1.0 green:0.0 blue:0.2 alpha:1.0]; break; // Caps: Red
        case 2: borderColor = [NSColor colorWithCalibratedRed:0.25 green:0.88 blue:0.0 alpha:1.0]; break; // WASD: Grass Green
        case 3: borderColor = [NSColor colorWithCalibratedRed:0.0 green:0.9 blue:1.0 alpha:1.0]; break; // QERF: Neon Cyan
        case 4: borderColor = [NSColor colorWithCalibratedRed:1.0 green:0.27 blue:0.0 alpha:1.0]; break; // Space: Neon Orange
        case 5: borderColor = [NSColor colorWithCalibratedRed:1.0 green:1.0 blue:0.0 alpha:1.0]; break; // G: Neon Yellow
        case 6: borderColor = [NSColor whiteColor]; break; // Shift: White
        default: borderColor = [NSColor colorWithCalibratedRed:0.235 green:0.251 blue:0.251 alpha:1.0]; break;
    }
    textColor = (category > 0) ? borderColor : [NSColor colorWithCalibratedRed:0.53 green:0.53 blue:0.53 alpha:1.0];

    NSBezierPath *path = [NSBezierPath bezierPathWithRoundedRect:rect xRadius:6.0 yRadius:6.0];
    [bgColor setFill];
    [path fill];
    [borderColor setStroke];
    [path setLineWidth:1.2];
    [path stroke];

    if (key.length > 0 && desc.length == 0) {
        // 하단 설명 없는 키캡: 수직 중앙 정렬로 상하 여백 넉넉하게 확보
        NSFont *keyFont = [NSFont boldSystemFontOfSize:12.0];
        NSDictionary *keyAttrs = @{ NSFontAttributeName: keyFont, NSForegroundColorAttributeName: textColor };
        NSSize keySize = [key sizeWithAttributes:keyAttrs];
        NSRect keyRect = NSMakeRect(rect.origin.x + (rect.size.width - keySize.width)/2.0, rect.origin.y + (rect.size.height - keySize.height)/2.0, keySize.width, keySize.height);
        [key drawInRect:keyRect withAttributes:keyAttrs];
    } else {
        if (key.length > 0) {
            NSFont *keyFont = [NSFont boldSystemFontOfSize:12.0];
            NSDictionary *keyAttrs = @{ NSFontAttributeName: keyFont, NSForegroundColorAttributeName: textColor };
            NSSize keySize = [key sizeWithAttributes:keyAttrs];
            NSRect keyRect = NSMakeRect(rect.origin.x + (rect.size.width - keySize.width)/2.0, rect.origin.y + rect.size.height - keySize.height - 6.0, keySize.width, keySize.height);
            [key drawInRect:keyRect withAttributes:keyAttrs];
        }

        if (desc.length > 0) {
            // 하단 설명 폰트 12.0pt 통일 + 상하 여백 확대 (6.0pt)
            NSFont *descFont = [NSFont systemFontOfSize:12.0 weight:NSFontWeightMedium];
            NSColor *descColor = (category > 0) ? [NSColor whiteColor] : [NSColor colorWithCalibratedRed:0.33 green:0.33 blue:0.33 alpha:1.0];
            NSDictionary *descAttrs = @{ NSFontAttributeName: descFont, NSForegroundColorAttributeName: descColor };
            NSSize descSize = [desc sizeWithAttributes:descAttrs];
            NSRect descRect = NSMakeRect(rect.origin.x + (rect.size.width - descSize.width)/2.0, rect.origin.y + 6.0, descSize.width, descSize.height);
            [desc drawInRect:descRect withAttributes:descAttrs];
        }
    }
}

static void drawHudButton(NSRect rect, NSString *label, NSColor *bgColor, NSColor *borderColor, NSColor *textColor) {
    NSBezierPath *path = [NSBezierPath bezierPathWithRoundedRect:rect xRadius:5.0 yRadius:5.0];
    [bgColor setFill];
    [path fill];
    [borderColor setStroke];
    [path setLineWidth:1.0];
    [path stroke];

    NSFont *font = [NSFont boldSystemFontOfSize:12.0];
    NSDictionary *attrs = @{ NSFontAttributeName: font, NSForegroundColorAttributeName: textColor };
    NSSize sz = [label sizeWithAttributes:attrs];
    NSRect r = NSMakeRect(rect.origin.x + (rect.size.width - sz.width)/2.0, rect.origin.y + (rect.size.height - sz.height)/2.0, sz.width, sz.height);
    [label drawInRect:r withAttributes:attrs];
}

static void drawMacToggleRow(NSRect rect, NSString *label, CGFloat animProgress, NSColor *textColor) {
    // 1. 버튼 배경 컨테이너 패널
    NSBezierPath *bgPath = [NSBezierPath bezierPathWithRoundedRect:rect xRadius:6.0 yRadius:6.0];
    [keysorButtonBgColor() setFill];
    [bgPath fill];

    // 테두리 색상 (OFF 어두운 테두리 -> ON 네온 그린 매끄러운 보간)
    CGFloat br = 0.24 + animProgress * (0.184 - 0.24);
    CGFloat bg = 0.25 + animProgress * (1.0 - 0.25);
    CGFloat bb = 0.25 + animProgress * (0.678 - 0.25);
    [[NSColor colorWithCalibratedRed:br green:bg blue:bb alpha:1.0] setStroke];
    [bgPath setLineWidth:1.0];
    [bgPath stroke];

    // 2. 좌측 텍스트 라벨 (12.0pt 폰트 통일, 좌측 정렬)
    NSFont *font = [NSFont boldSystemFontOfSize:12.0];
    NSDictionary *attrs = @{ NSFontAttributeName: font, NSForegroundColorAttributeName: textColor };
    NSSize sz = [label sizeWithAttributes:attrs];
    NSRect textRect = NSMakeRect(rect.origin.x + 8.0, rect.origin.y + (rect.size.height - sz.height) / 2.0, sz.width, sz.height);
    [label drawInRect:textRect withAttributes:attrs];

    // 3. 우측 맥 스타일 토글 스위치 캡슐 (너비 32pt, 높이 16pt)
    CGFloat switchW = 32.0;
    CGFloat switchH = 16.0;
    NSRect switchRect = NSMakeRect(rect.origin.x + rect.size.width - switchW - 6.0, rect.origin.y + (rect.size.height - switchH) / 2.0, switchW, switchH);

    // 캡슐 배경 색상 (OFF 어두운 그레이 -> ON 네온 그린 매끄러운 보간)
    CGFloat cr = 0.25 + animProgress * (0.184 - 0.25);
    CGFloat cg = 0.25 + animProgress * (1.0 - 0.25);
    CGFloat cb = 0.25 + animProgress * (0.678 - 0.25);
    NSColor *capsuleColor = [NSColor colorWithCalibratedRed:cr green:cg blue:cb alpha:1.0];

    NSBezierPath *capsulePath = [NSBezierPath bezierPathWithRoundedRect:switchRect xRadius:8.0 yRadius:8.0];
    [capsuleColor setFill];
    [capsulePath fill];

    // 4. 원형 슬라이더 노브 (반지름 6pt -> 12x12, X좌표 매끄러운 슬라이딩 모션)
    CGFloat knobSize = 12.0;
    CGFloat minKnobX = switchRect.origin.x + 2.0;
    CGFloat maxKnobX = switchRect.origin.x + switchW - knobSize - 2.0;
    CGFloat knobX = minKnobX + animProgress * (maxKnobX - minKnobX);
    CGFloat knobY = switchRect.origin.y + 2.0;
    NSRect knobRect = NSMakeRect(knobX, knobY, knobSize, knobSize);

    NSBezierPath *knobPath = [NSBezierPath bezierPathWithOvalInRect:knobRect];
    [[NSColor whiteColor] setFill];
    [knobPath fill];
}

// 1. HUD 메인 배경 패널 렌더링
- (void)drawHudBackgroundPanel:(NSRect)bounds {
    NSBezierPath *bgPath = [NSBezierPath bezierPathWithRoundedRect:bounds xRadius:16.0 yRadius:16.0];
    [keysorPanelBgColor(0.95) setFill];
    [bgPath fill];

    // 우하단 3D 그림자 테두리 (Dark Forest Green)
    NSRect shadowBounds = NSMakeRect(bounds.origin.x + 1, bounds.origin.y - 1, bounds.size.width, bounds.size.height);
    NSBezierPath *shadowPath = [NSBezierPath bezierPathWithRoundedRect:shadowBounds xRadius:16.0 yRadius:16.0];
    [keysorDarkGreenShadowColor(0.8) setStroke];
    [shadowPath setLineWidth:2.0];
    [shadowPath stroke];

    // 좌상단 메인 테두리 (Keysor Signature Neon Green)
    [keysorNeonGreenColor(0.9) setStroke];
    [bgPath setLineWidth:2.0];
    [bgPath stroke];
}

// 2. 타이틀 헤더 및 상단 컨트롤 버튼 렌더링
- (void)drawHudHeader:(CGFloat)h {
    NSString *titleStr = self.langIsEnglish ? @"Keysor, Keyboard & Cursor as One!" : @"Keysor, 키보드와 커서를 하나로!";
    NSFont *titleFont = [NSFont boldSystemFontOfSize:18.0];

    // 타이틀 그림자 레이어
    NSDictionary *titleShadowAttrs = @{ NSFontAttributeName: titleFont, NSForegroundColorAttributeName: keysorDarkGreenShadowColor(1.0) };
    [titleStr drawAtPoint:NSMakePoint(31, h - 30 - 23) withAttributes:titleShadowAttrs];

    // 타이틀 메인 레이어
    NSDictionary *titleAttrs = @{ NSFontAttributeName: titleFont, NSForegroundColorAttributeName: keysorNeonGreenColor(1.0) };
    [titleStr drawAtPoint:NSMakePoint(30, h - 30 - 24) withAttributes:titleAttrs];

    // Top Right 컨트롤 버튼들 (Buy Pro, License, KO/EN, Minimize, Close)
    drawHudButton(NSMakeRect(430, h - 30 - 24, 110, 24), self.langIsEnglish ? @"Buy Pro" : @"프로 결제하기", keysorButtonBgColor(), keysorButtonBorderColor(), [NSColor colorWithCalibratedRed:0.53 green:0.53 blue:0.53 alpha:1.0]);
    drawHudButton(NSMakeRect(546, h - 30 - 24, 110, 24), self.langIsEnglish ? @"License" : @"라이선스 등록", keysorButtonBgColor(), keysorButtonBorderColor(), [NSColor colorWithCalibratedRed:0.53 green:0.53 blue:0.53 alpha:1.0]);
    drawHudButton(NSMakeRect(662, h - 30 - 24, 50, 24), self.langIsEnglish ? @"KO" : @"EN", keysorButtonBgColor(), keysorNeonGreenColor(1.0), keysorNeonGreenColor(1.0));
    drawHudButton(NSMakeRect(742, h - 10 - 20, 24, 20), @"-", keysorButtonBgColor(), keysorButtonBorderColor(), [NSColor whiteColor]);
    drawHudButton(NSMakeRect(772, h - 10 - 20, 24, 20), @"X", keysorButtonBgColor(), keysorButtonBorderColor(), [NSColor colorWithCalibratedRed:1.0 green:0.27 blue:0.0 alpha:1.0]);
}

// 3. 5열 온스크린 키보드 가이드 레이아웃 렌더링
- (void)drawOnscreenKeyboardLayout:(CGFloat)h {
    NSString *qDesc = self.langIsEnglish ? @"Scl ◀" : @"◀스크롤";
    NSString *wDesc = self.langIsEnglish ? @"Up ▲" : @"▲ 이동";
    NSString *eDesc = self.langIsEnglish ? @"Scl ▶" : @"스크롤▶";
    NSString *rDesc = self.langIsEnglish ? @"Whl ▲" : @"휠▲";
    NSString *capsDesc = self.langIsEnglish ? @"ON/OFF" : @"온/오프";
    NSString *aDesc = self.langIsEnglish ? @"Left ◀" : @"◀ 이동";
    NSString *sDesc = self.langIsEnglish ? @"Down ▼" : @"▼ 이동";
    NSString *dDesc = self.langIsEnglish ? @"Right ▶" : @"▶ 이동";
    NSString *fDesc = self.langIsEnglish ? @"Whl ▼" : @"휠▼";
    NSString *gDesc = self.langIsEnglish ? @"R-Clk" : @"우클릭";
    NSString *spaceDesc = self.langIsEnglish ? @"Left Click (1:L / 2:Db / Hold:Drag)" : @"좌클릭 (1:일반 / 2:더블 / 홀드:드래그)";

    // Row 1: Numbers (Y = 80)
    drawKeyCap(NSMakeRect(30, h - 80 - 48, 48, 48), @"~", @"", 0);
    drawKeyCap(NSMakeRect(84, h - 80 - 48, 48, 48), @"1", @"", 0);
    drawKeyCap(NSMakeRect(138, h - 80 - 48, 48, 48), @"2", @"", 0);
    drawKeyCap(NSMakeRect(192, h - 80 - 48, 48, 48), @"3", @"", 0);
    drawKeyCap(NSMakeRect(246, h - 80 - 48, 48, 48), @"4", @"", 0);
    drawKeyCap(NSMakeRect(300, h - 80 - 48, 48, 48), @"5", @"", 0);
    drawKeyCap(NSMakeRect(354, h - 80 - 48, 48, 48), @"6", @"", 0);
    drawKeyCap(NSMakeRect(408, h - 80 - 48, 48, 48), @"7", @"", 0);
    drawKeyCap(NSMakeRect(462, h - 80 - 48, 48, 48), @"8", @"", 0);
    drawKeyCap(NSMakeRect(516, h - 80 - 48, 48, 48), @"9", @"", 0);
    drawKeyCap(NSMakeRect(570, h - 80 - 48, 48, 48), @"0", @"", 0);

    // Row 2: Q Row (Y = 134)
    drawKeyCap(NSMakeRect(30, h - 134 - 48, 72, 48), @"Tab", @"", 0);
    drawKeyCap(NSMakeRect(108, h - 134 - 48, 48, 48), @"Q", qDesc, 3);
    drawKeyCap(NSMakeRect(162, h - 134 - 48, 48, 48), @"W", wDesc, 2);
    drawKeyCap(NSMakeRect(216, h - 134 - 48, 48, 48), @"E", eDesc, 3);
    drawKeyCap(NSMakeRect(270, h - 134 - 48, 48, 48), @"R", rDesc, 3);
    drawKeyCap(NSMakeRect(324, h - 134 - 48, 48, 48), @"T", @"", 0);
    drawKeyCap(NSMakeRect(378, h - 134 - 48, 48, 48), @"Y", @"", 0);
    drawKeyCap(NSMakeRect(432, h - 134 - 48, 48, 48), @"U", @"", 0);
    drawKeyCap(NSMakeRect(486, h - 134 - 48, 48, 48), @"I", @"", 0);
    drawKeyCap(NSMakeRect(540, h - 134 - 48, 48, 48), @"O", @"", 0);

    // Row 3: A Row (Y = 188)
    drawKeyCap(NSMakeRect(30, h - 188 - 48, 84, 48), @"Caps", capsDesc, 1);
    drawKeyCap(NSMakeRect(120, h - 188 - 48, 48, 48), @"A", aDesc, 2);
    drawKeyCap(NSMakeRect(174, h - 188 - 48, 48, 48), @"S", sDesc, 2);
    drawKeyCap(NSMakeRect(228, h - 188 - 48, 48, 48), @"D", dDesc, 2);
    drawKeyCap(NSMakeRect(282, h - 188 - 48, 48, 48), @"F", fDesc, 3);
    drawKeyCap(NSMakeRect(336, h - 188 - 48, 48, 48), @"G", gDesc, 5);
    drawKeyCap(NSMakeRect(390, h - 188 - 48, 48, 48), @"H", @"", 0);
    drawKeyCap(NSMakeRect(444, h - 188 - 48, 48, 48), @"J", self.langIsEnglish ? @"◀Tab" : @"◀크롬탭", 6);
    drawKeyCap(NSMakeRect(498, h - 188 - 48, 48, 48), @"K", self.langIsEnglish ? @"Tab▶" : @"크롬탭▶", 6);
    drawKeyCap(NSMakeRect(552, h - 188 - 48, 48, 48), @"L", @"", 0);

    // Row 4: Z Row & Shift (Y = 242)
    drawKeyCap(NSMakeRect(30, h - 242 - 48, 102, 48), @"Shift", self.langIsEnglish ? @"+Q/E Back/Fwd" : @"+Q/E 뒤로/앞으로", 6);
    drawKeyCap(NSMakeRect(138, h - 242 - 48, 48, 48), @"Z", @"", 0);
    drawKeyCap(NSMakeRect(192, h - 242 - 48, 48, 48), @"X", @"", 0);
    drawKeyCap(NSMakeRect(246, h - 242 - 48, 48, 48), @"C", @"", 0);
    drawKeyCap(NSMakeRect(300, h - 242 - 48, 48, 48), @"V", @"", 0);
    drawKeyCap(NSMakeRect(354, h - 242 - 48, 48, 48), @"B", @"", 0);
    drawKeyCap(NSMakeRect(408, h - 242 - 48, 48, 48), @"N", @"", 0);
    drawKeyCap(NSMakeRect(462, h - 242 - 48, 48, 48), @"M", @"", 0);
    drawKeyCap(NSMakeRect(516, h - 242 - 48, 48, 48), @"<", @"", 0);

    // Row 5: Modifier & Space (Y = 296)
    drawKeyCap(NSMakeRect(30, h - 296 - 48, 48, 48), @"Ctrl", @"", 0);
    drawKeyCap(NSMakeRect(84, h - 296 - 48, 48, 48), @"Cmd", @"", 0);
    drawKeyCap(NSMakeRect(138, h - 296 - 48, 48, 48), @"Opt", @"", 0);
    drawKeyCap(NSMakeRect(192, h - 296 - 48, 264, 48), @"Spacebar", spaceDesc, 4);
    drawKeyCap(NSMakeRect(462, h - 296 - 48, 48, 48), @"Opt", @"", 0);
    drawKeyCap(NSMakeRect(516, h - 296 - 48, 48, 48), @"Cmd", @"", 0);
}

// 4. SPEED SENS 패널 렌더링 (키보드 영역 높이 264pt와 100% 동일 정밀 동기화)
- (void)drawSpeedSensPanel:(CGFloat)h {
    NSRect sensPanelRect = NSMakeRect(634, h - 80 - 264, 148, 264);
    NSBezierPath *sensPath = [NSBezierPath bezierPathWithRoundedRect:sensPanelRect xRadius:10.0 yRadius:10.0];
    [[NSColor colorWithCalibratedRed:0.09 green:0.09 blue:0.09 alpha:1.0] setFill];
    [sensPath fill];

    // SPEED SENS 패널 3D 그림자 테두리
    NSRect sensShadowRect = NSMakeRect(sensPanelRect.origin.x + 1, sensPanelRect.origin.y - 1, sensPanelRect.size.width, sensPanelRect.size.height);
    NSBezierPath *sensShadowPath = [NSBezierPath bezierPathWithRoundedRect:sensShadowRect xRadius:10.0 yRadius:10.0];
    [keysorDarkGreenShadowColor(0.8) setStroke];
    [sensShadowPath setLineWidth:1.5];
    [sensShadowPath stroke];

    // SPEED SENS 패널 메인 테두리
    [keysorNeonGreenColor(0.8) setStroke];
    [sensPath setLineWidth:1.5];
    [sensPath stroke];

    // SPEED SENS 타이틀 (12.0pt 폰트 통일)
    NSFont *sensFont = [NSFont boldSystemFontOfSize:12.0];
    NSDictionary *sensTitleAttrs = @{ NSFontAttributeName: sensFont, NSForegroundColorAttributeName: keysorNeonGreenColor(1.0) };
    NSSize sensTitleSz = [@"SPEED SENS" sizeWithAttributes:sensTitleAttrs];
    CGFloat sensTitleX = 634.0 + (148.0 - sensTitleSz.width) / 2.0;

    NSDictionary *sensTitleShadowAttrs = @{ NSFontAttributeName: sensFont, NSForegroundColorAttributeName: keysorDarkGreenShadowColor(1.0) };
    [@"SPEED SENS" drawAtPoint:NSMakePoint(sensTitleX + 1.0, h - 88 - 15) withAttributes:sensTitleShadowAttrs];
    [@"SPEED SENS" drawAtPoint:NSMakePoint(sensTitleX, h - 88 - 16) withAttributes:sensTitleAttrs];

    // 수치 디스플레이 (센서값 및 상세보기 수치 위치 하향 조정)
    if (self.showAllDetails) {
        NSFont *detailFont = [NSFont boldSystemFontOfSize:12.0];
        NSDictionary *detailAttrs = @{ NSFontAttributeName: detailFont, NSForegroundColorAttributeName: [NSColor whiteColor] };

        NSString *l1 = [NSString stringWithFormat:@"Base : %.1f", self.baseSpeed > 0 ? self.baseSpeed : 1.5];
        NSString *l2 = @"Max  : 30.0";
        NSString *l3 = @"Accel: 1.5x";

        [l1 drawAtPoint:NSMakePoint(648, h - 108 - 13) withAttributes:detailAttrs];
        [l2 drawAtPoint:NSMakePoint(648, h - 125 - 13) withAttributes:detailAttrs];
        [l3 drawAtPoint:NSMakePoint(648, h - 142 - 13) withAttributes:detailAttrs];
    } else {
        NSString *spdStr = [NSString stringWithFormat:@"%.1f", self.baseSpeed > 0 ? self.baseSpeed : 1.5];
        NSDictionary *spdAttrs = @{ NSFontAttributeName: [NSFont boldSystemFontOfSize:24.0], NSForegroundColorAttributeName: [NSColor whiteColor] };
        NSSize spdSz = [spdStr sizeWithAttributes:spdAttrs];
        [spdStr drawAtPoint:NSMakePoint(634 + (148 - spdSz.width)/2.0, h - 120 - 24) withAttributes:spdAttrs];
    }

    // Row 1: - / + 속도 조절 버튼 (3px 상향 조정: Y=h-162)
    NSColor *minusBg = (self.pressedButtonTag == 1) ? keysorNeonGreenColor(0.4) : keysorButtonBgColor();
    NSColor *minusBorder = (self.pressedButtonTag == 1) ? keysorNeonGreenColor(1.0) : keysorButtonBorderColor();
    NSColor *minusTxt = (self.pressedButtonTag == 1) ? [NSColor blackColor] : [NSColor whiteColor];
    drawHudButton(NSMakeRect(644, h - 162 - 28, 61, 28), @"-", minusBg, minusBorder, minusTxt);

    NSColor *plusBg = (self.pressedButtonTag == 2) ? keysorNeonGreenColor(0.4) : keysorButtonBgColor();
    NSColor *plusBorder = (self.pressedButtonTag == 2) ? keysorNeonGreenColor(1.0) : keysorButtonBorderColor();
    NSColor *plusTxt = (self.pressedButtonTag == 2) ? [NSColor blackColor] : [NSColor whiteColor];
    drawHudButton(NSMakeRect(711, h - 162 - 28, 61, 28), @"+", plusBg, plusBorder, plusTxt);

    // Row 2: 그리드 모드 (3px 상향 조정: Y=h-198)
    drawMacToggleRow(NSMakeRect(644, h - 198 - 28, 128, 28), self.langIsEnglish ? @"Grid Mode" : @"그리드 모드", self.animGridProgress, (self.animGridProgress > 0.5) ? keysorNeonGreenColor(1.0) : [NSColor whiteColor]);

    // Row 3: 자석 모드 (3px 상향 조정: Y=h-234)
    drawMacToggleRow(NSMakeRect(644, h - 234 - 28, 128, 28), self.langIsEnglish ? @"Magnet Mode" : @"자석 모드", self.animMagnetProgress, (self.animMagnetProgress > 0.5) ? keysorNeonGreenColor(1.0) : [NSColor whiteColor]);

    // Row 4: 자동 실행 (3px 상향 조정: Y=h-270)
    drawMacToggleRow(NSMakeRect(644, h - 270 - 28, 128, 28), self.langIsEnglish ? @"Auto-Start" : @"자동 실행", self.animAutoStartProgress, (self.animAutoStartProgress > 0.5) ? keysorNeonGreenColor(1.0) : [NSColor whiteColor]);

    // Row 5: 상세보기 / 기본보기 토글 버튼 (3px 상향 조정: Y=h-306, 하단 마진 정확히 10pt 확보)
    NSString *btnLabel = self.showAllDetails ? (self.langIsEnglish ? @"Standard" : @"기본보기") : (self.langIsEnglish ? @"Details" : @"상세보기");
    NSColor *detailsBg = self.showAllDetails ? keysorNeonGreenColor(0.35) : keysorButtonBgColor();
    NSColor *detailsBorder = self.showAllDetails ? keysorNeonGreenColor(1.0) : keysorButtonBorderColor();
    NSColor *detailsTxt = self.showAllDetails ? [NSColor blackColor] : [NSColor whiteColor];
    drawHudButton(NSMakeRect(644, h - 306 - 28, 128, 28), btnLabel, detailsBg, detailsBorder, detailsTxt);
}

// 5. 하단 사용 가이드 문구 렌더링
- (void)drawFooterNotice:(CGFloat)h {
    NSString *info1 = self.langIsEnglish ? @"※ All alphabet typing is blocked during mouse mode (except Ctrl/Alt/Cmd shortcuts)." : @"※ 마우스 모드 중 모든 알파벳 키 타이핑은 차단됩니다 (Ctrl, Alt, Cmd 단축키 예외 허용).";
    NSString *info2 = self.langIsEnglish ? @"• Press Caps Lock again to return to normal keyboard mode." : @"• Caps Lock을 한 번 더 누르면 일반 키보드 상태로 복귀합니다.";
    NSString *info3 = self.langIsEnglish ? @"• Automatically minimizes when mouse mode is active." : @"• 마우스 활성화 시 자동으로 최소화 됩니다.";

    NSDictionary *infoAttrs = @{ NSFontAttributeName: [NSFont systemFontOfSize:12.0], NSForegroundColorAttributeName: [NSColor colorWithCalibratedRed:0.6 green:0.6 blue:0.6 alpha:1.0] };
    [info1 drawAtPoint:NSMakePoint(30, h - 365 - 14) withAttributes:infoAttrs];
    [info2 drawAtPoint:NSMakePoint(30, h - 388 - 14) withAttributes:infoAttrs];
    [info3 drawAtPoint:NSMakePoint(30, h - 411 - 14) withAttributes:infoAttrs];
}

- (void)drawRect:(NSRect)dirtyRect {
    [super drawRect:dirtyRect];

    NSRect bounds = [self bounds];
    CGFloat contentHeight = bounds.size.height;

    // 모듈화된 서브 렌더러 순차 호출 (단일 책임 원칙 준수)
    [self drawHudBackgroundPanel:bounds];
    [self drawHudHeader:contentHeight];
    [self drawOnscreenKeyboardLayout:contentHeight];
    [self drawSpeedSensPanel:contentHeight];
    [self drawFooterNotice:contentHeight];
}

// 상단 헤더 컨트롤 버튼 클릭 핸들러
- (BOOL)handleHeaderButtonClicks:(NSPoint)p contentHeight:(CGFloat)h {
    // 언어 변경 버튼
    if (NSPointInRect(p, NSMakeRect(662, h - 30 - 24, 50, 24))) {
        self.langIsEnglish = !self.langIsEnglish;
        [self setNeedsDisplay:YES];
        keysor_macos_on_lang_toggle();
        return YES;
    }

    // 최소화 버튼
    if (NSPointInRect(p, NSMakeRect(742, h - 10 - 20, 24, 20))) {
        [[self window] orderOut:nil];
        return YES;
    }

    // 닫기 버튼
    if (NSPointInRect(p, NSMakeRect(772, h - 10 - 20, 24, 20))) {
        NSLog(@"[Keysor ObjC] HUD Close X button clicked, exiting Keysor...");
        exit(0);
        return YES;
    }

    // Pro 구매 버튼
    if (NSPointInRect(p, NSMakeRect(430, h - 30 - 24, 110, 24))) {
        [[NSWorkspace sharedWorkspace] openURL:[NSURL URLWithString:@"https://keysor.vercel.app/#pricing"]];
        return YES;
    }

    return NO;
}

// SPEED SENS 패널 버튼 클릭 핸들러
- (BOOL)handleSpeedPanelButtonClicks:(NSPoint)p contentHeight:(CGFloat)h {
    // - 속도 감소 (0.1 미세 조절)
    if (NSPointInRect(p, NSMakeRect(644, h - 162 - 28, 61, 28))) {
        self.pressedButtonTag = 1;
        self.baseSpeed = keysor_macos_on_speed_change(-0.1);
        [self setNeedsDisplay:YES];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 150 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
            self.pressedButtonTag = 0;
            [self setNeedsDisplay:YES];
        });
        return YES;
    }

    // + 속도 증가 (0.1 미세 조절)
    if (NSPointInRect(p, NSMakeRect(711, h - 162 - 28, 61, 28))) {
        self.pressedButtonTag = 2;
        self.baseSpeed = keysor_macos_on_speed_change(0.1);
        [self setNeedsDisplay:YES];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 150 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
            self.pressedButtonTag = 0;
            [self setNeedsDisplay:YES];
        });
        return YES;
    }

    // 그리드 모드 토글 버튼
    if (NSPointInRect(p, NSMakeRect(644, h - 198 - 28, 128, 28))) {
        self.gridModeEnabled = keysor_macos_on_grid_toggle();
        [self startToggleAnimTimer];
        return YES;
    }

    // 자석 모드 토글 버튼
    if (NSPointInRect(p, NSMakeRect(644, h - 234 - 28, 128, 28))) {
        self.magnetModeEnabled = keysor_macos_on_magnet_toggle();
        [self startToggleAnimTimer];
        return YES;
    }

    // 자동실행 토글 버튼
    if (NSPointInRect(p, NSMakeRect(644, h - 270 - 28, 128, 28))) {
        self.autoStartEnabled = keysor_macos_on_autostart_toggle();
        [self startToggleAnimTimer];
        return YES;
    }

    // 상세보기 / 기본보기 토글 버튼
    if (NSPointInRect(p, NSMakeRect(644, h - 306 - 28, 128, 28))) {
        self.showAllDetails = !self.showAllDetails;
        [self setNeedsDisplay:YES];
        return YES;
    }

    return NO;
}

- (void)startToggleAnimTimer {
    if (self.animTimer != nil) return;
    self.animTimer = [NSTimer scheduledTimerWithTimeInterval:1.0/60.0 repeats:YES block:^(NSTimer *timer) {
        CGFloat targetGrid = self.gridModeEnabled ? 1.0 : 0.0;
        CGFloat targetMag = self.magnetModeEnabled ? 1.0 : 0.0;
        CGFloat targetAuto = self.autoStartEnabled ? 1.0 : 0.0;

        self.animGridProgress += (targetGrid - self.animGridProgress) * 0.35;
        self.animMagnetProgress += (targetMag - self.animMagnetProgress) * 0.35;
        self.animAutoStartProgress += (targetAuto - self.animAutoStartProgress) * 0.35;

        if (fabs(targetGrid - self.animGridProgress) < 0.01) self.animGridProgress = targetGrid;
        if (fabs(targetMag - self.animMagnetProgress) < 0.01) self.animMagnetProgress = targetMag;
        if (fabs(targetAuto - self.animAutoStartProgress) < 0.01) self.animAutoStartProgress = targetAuto;

        [self setNeedsDisplay:YES];

        if (self.animGridProgress == targetGrid &&
            self.animMagnetProgress == targetMag &&
            self.animAutoStartProgress == targetAuto) {
            [self.animTimer invalidate];
            self.animTimer = nil;
        }
    }];
}

// 창 마우스 드래그 및 버튼 클릭 이벤트 처리
- (void)mouseDown:(NSEvent *)event {
    NSPoint p = [self convertPoint:[event locationInWindow] fromView:nil];
    CGFloat h = [self bounds].size.height;

    // 1. 상단 헤더 버튼 클릭 처리
    if ([self handleHeaderButtonClicks:p contentHeight:h]) {
        return;
    }

    // 2. SPEED SENS 패널 버튼 클릭 처리
    if ([self handleSpeedPanelButtonClicks:p contentHeight:h]) {
        return;
    }

    // 3. 윈도우 배경 드래그 이동 시작 포인트 기록
    self.dragStartPoint = [NSEvent mouseLocation];
}

- (void)mouseDragged:(NSEvent *)event {
    NSPoint current = [NSEvent mouseLocation];
    NSWindow *win = [self window];
    NSPoint origin = [win frame].origin;
    origin.x += (current.x - self.dragStartPoint.x);
    origin.y += (current.y - self.dragStartPoint.y);
    [win setFrameOrigin:origin];
    self.dragStartPoint = current;
}

@end

@interface KeysorHudWindow : NSWindow
@end

@implementation KeysorHudWindow
- (BOOL)canBecomeKeyWindow {
    return YES;
}
- (BOOL)canBecomeMainWindow {
    return YES;
}
@end

static NSWindow *g_overlayWindow = nil;
static KeysorCursorView *g_cursorView = nil;

static KeysorHudWindow *g_hudWindow = nil;
static KeysorHudView *g_hudView = nil;

@interface KeysorAppDelegate : NSObject <NSApplicationDelegate>
@end

void keysor_macos_show_hud(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_hudWindow != nil) {
            if (g_hudView != nil) {
                g_hudView.baseSpeed = keysor_macos_get_base_speed();
                [g_hudView setNeedsDisplay:YES];
            }
            NSRect mainScreenFrame = [[NSScreen mainScreen] frame];
            CGFloat hudX = (mainScreenFrame.size.width - 808.0) / 2.0;
            CGFloat hudY = (mainScreenFrame.size.height - 452.0) / 2.0 - 100.0;
            [g_hudWindow setFrameOrigin:NSMakePoint(hudX, hudY)];
            [g_hudWindow setAlphaValue:1.0];
            [g_hudWindow setLevel:NSModalPanelWindowLevel];
            [g_hudWindow makeKeyAndOrderFront:nil];
            [g_hudWindow orderFrontRegardless];
            [NSApp activateIgnoringOtherApps:YES];
            [g_hudWindow display];
        }
    });
}

void keysor_macos_post_show_hud_notification(void) {
    [[NSDistributedNotificationCenter defaultCenter] postNotificationName:@"KeysorShowHudNotification"
                                                                   object:nil
                                                                 userInfo:nil
                                                       deliverImmediately:YES];
}

@implementation KeysorAppDelegate

- (void)handleReopenEvent:(NSAppleEventDescriptor *)event withReplyEvent:(NSAppleEventDescriptor *)replyEvent {
    NSLog(@"[Keysor ObjC] handleReopenEvent (Dock Click) received!");
    keysor_macos_show_hud();
}

- (BOOL)applicationShouldHandleReopen:(NSApplication *)sender hasVisibleWindows:(BOOL)flag {
    NSLog(@"[Keysor ObjC] applicationShouldHandleReopen called! hasVisibleWindows=%d", flag);
    keysor_macos_show_hud();
    return YES;
}

- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    NSLog(@"[Keysor ObjC] applicationShouldTerminate called, exiting process...");
    exit(0);
    return NSTerminateNow;
}

- (void)applicationWillTerminate:(NSNotification *)notification {
    NSLog(@"[Keysor ObjC] applicationWillTerminate called, exiting process...");
    exit(0);
}

@end

static KeysorAppDelegate *g_appDelegate = nil;

void keysor_macos_pump_events(void) {
    @autoreleasepool {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.001, false);
        NSEvent *event;
        while ((event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                           untilDate:[NSDate distantPast]
                                              inMode:NSDefaultRunLoopMode
                                             dequeue:YES])) {
            [NSApp sendEvent:event];
            [NSApp updateWindows];
        }
    }
}

void keysor_macos_ui_init(void) {
    if (g_overlayWindow != nil) return;

    void (^initBlock)(void) = ^{
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        g_appDelegate = [[KeysorAppDelegate alloc] init];
        [NSApp setDelegate:g_appDelegate];
        [NSApp finishLaunching];

        [[NSAppleEventManager sharedAppleEventManager] setEventHandler:g_appDelegate
                                                           andSelector:@selector(handleReopenEvent:withReplyEvent:)
                                                         forEventClass:kCoreEventClass
                                                            andEventID:kAEReopenApplication];

        [[NSDistributedNotificationCenter defaultCenter] addObserverForName:@"KeysorShowHudNotification"
                                                                      object:nil
                                                                       queue:[NSOperationQueue mainQueue]
                                                                  usingBlock:^(NSNotification *note) {
            keysor_macos_show_hud();
        }];

        // 1. 커서 오버레이 윈도우 생성 (36x36 Baseline)
        NSRect frame = NSMakeRect(0, 0, 36, 36);
        g_overlayWindow = [[NSWindow alloc] initWithContentRect:frame
                                                      styleMask:NSWindowStyleMaskBorderless
                                                        backing:NSBackingStoreBuffered
                                                          defer:NO];

        [g_overlayWindow setBackgroundColor:[NSColor clearColor]];
        [g_overlayWindow setOpaque:NO];
        [g_overlayWindow setHasShadow:NO];
        [g_overlayWindow setLevel:NSScreenSaverWindowLevel]; // 1000 (Topmost ScreenSaver Level)
        [g_overlayWindow setHidesOnDeactivate:NO];
        [g_overlayWindow setCanHide:NO];
        [g_overlayWindow setCollectionBehavior:NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorStationary | NSWindowCollectionBehaviorIgnoresCycle];
        [g_overlayWindow setIgnoresMouseEvents:YES];

        g_cursorView = [[KeysorCursorView alloc] initWithFrame:frame];
        [g_overlayWindow setContentView:g_cursorView];

        // 2. HUD 키보드 가이드 & 설정 팝업 윈도우 생성 (808x452, 화면 아래쪽 배치)
        NSRect mainScreenFrame = [[NSScreen mainScreen] frame];
        CGFloat hudX = (mainScreenFrame.size.width - 808.0) / 2.0;
        CGFloat hudY = (mainScreenFrame.size.height - 452.0) / 2.0 - 100.0;
        NSRect hudFrame = NSMakeRect(hudX, hudY, 808.0, 452.0);

        g_hudWindow = [[KeysorHudWindow alloc] initWithContentRect:hudFrame
                                                         styleMask:NSWindowStyleMaskBorderless
                                                           backing:NSBackingStoreBuffered
                                                             defer:NO];
        [g_hudWindow setBackgroundColor:[NSColor clearColor]];
        [g_hudWindow setOpaque:NO];
        [g_hudWindow setHasShadow:YES];
        [g_hudWindow setLevel:NSStatusWindowLevel]; // 25
        [g_hudWindow setHidesOnDeactivate:NO];
        [g_hudWindow setCollectionBehavior:NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorStationary];

        g_hudView = [[KeysorHudView alloc] initWithFrame:NSMakeRect(0, 0, 808.0, 452.0)];
        g_hudView.baseSpeed = keysor_macos_get_base_speed();
        g_hudView.langIsEnglish = NO;
        g_hudView.autoStartEnabled = keysor_macos_check_autostart_status();
        g_hudView.magnetModeEnabled = keysor_macos_check_magnet_status();
        g_hudView.gridModeEnabled = keysor_macos_check_grid_status();
        g_hudView.animGridProgress = g_hudView.gridModeEnabled ? 1.0 : 0.0;
        g_hudView.animMagnetProgress = g_hudView.magnetModeEnabled ? 1.0 : 0.0;
        g_hudView.animAutoStartProgress = g_hudView.autoStartEnabled ? 1.0 : 0.0;
        [g_hudWindow setContentView:g_hudView];

        // 최초 구동 시 팝업 윈도우 화면에 표시
        [g_hudWindow orderFrontRegardless];
        NSLog(@"[Keysor Native ObjC UI] Keysor Overlay Window & HUD Settings Popup Window initialized successfully.");
    };

    if ([NSThread isMainThread]) {
        initBlock();
    } else {
        dispatch_async(dispatch_get_main_queue(), initBlock);
    }
}

void keysor_macos_ui_show(bool visible) {
    void (^showBlock)(void) = ^{
        if (g_overlayWindow == nil) return;
        if (visible) {
            [g_overlayWindow setAlphaValue:0.95];
            NSArray *screens = [NSScreen screens];
            NSRect primaryBounds = (screens.count > 0) ? [[screens firstObject] frame] : [[NSScreen mainScreen] frame];
            [g_overlayWindow setLevel:NSScreenSaverWindowLevel];
            [g_overlayWindow orderFrontRegardless];
            [g_overlayWindow display];

            // 마우스 모드 활성화 시 HUD 팝업 윈도우 가리기
            if (g_hudWindow != nil) {
                [g_hudWindow orderOut:nil];
            }
        } else {
            [g_overlayWindow orderOut:nil];
        }
    };

    if ([NSThread isMainThread]) {
        showBlock();
    } else {
        dispatch_async(dispatch_get_main_queue(), showBlock);
    }
}

void keysor_macos_ui_update_pos(double x, double y) {
    void (^posBlock)(void) = ^{
        if (g_overlayWindow == nil) return;
        // 다중/외부 모니터 환경에서도 주 디스플레이(screens[0]) 기준의 Y축 좌표계를 항상 일정하게 유지 (Item 1)
        NSArray *screens = [NSScreen screens];
        NSRect primaryBounds = (screens.count > 0) ? [[screens firstObject] frame] : [[NSScreen mainScreen] frame];
        double screen_h = primaryBounds.size.height;

        // 커서 핫스팟 (16, 16) 정밀 동기화 및 레티나 픽셀 스냅 라운딩
        NSPoint origin = NSMakePoint(round(x - 16.0), round(screen_h - y - 20.0));
        [g_overlayWindow setFrameOrigin:origin];

        // 자석 모드 흡착 시 커서 색상 시안(Cyan)으로 실시간 전환
        BOOL isSnapped = keysor_macos_check_is_snapped();
        if (g_cursorView != nil) {
            if (g_cursorView.cursorState == 0 || g_cursorView.cursorState == 5) {
                int newState = isSnapped ? 5 : 0;
                if (g_cursorView.cursorState != newState) {
                    g_cursorView.cursorState = newState;
                    [g_cursorView setNeedsDisplay:YES];
                }
            }
        }
    };

    if ([NSThread isMainThread]) {
        posBlock();
    } else {
        dispatch_async(dispatch_get_main_queue(), posBlock);
    }
}

static uint64_t g_clickMotionSeq = 0;

void keysor_macos_ui_set_click_motion(int click_type) {
    void (^motionBlock)(void) = ^{
        if (g_cursorView == nil) return;
        int targetState = click_type;
        if (click_type == 0) {
            BOOL isSnapped = keysor_macos_check_is_snapped();
            targetState = isSnapped ? 5 : 0;
        }

        if (g_cursorView.cursorState != targetState) {
            g_cursorView.cursorState = targetState;
            [g_cursorView setNeedsDisplay:YES];
        }

        uint64_t currentSeq = ++g_clickMotionSeq;

        if (click_type > 0 && click_type != 5) {
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 180 * NSEC_PER_MSEC), dispatch_get_main_queue(), ^{
                if (g_cursorView != nil && g_clickMotionSeq == currentSeq) {
                    BOOL isSnapped = keysor_macos_check_is_snapped();
                    int resetState = isSnapped ? 5 : 0;
                    if (g_cursorView.cursorState != resetState) {
                        g_cursorView.cursorState = resetState;
                        [g_cursorView setNeedsDisplay:YES];
                    }
                }
            });
        }
    };

    if ([NSThread isMainThread]) {
        motionBlock();
    } else {
        dispatch_async(dispatch_get_main_queue(), motionBlock);
    }
}

void keysor_macos_ui_update_speed_display(double speed) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_hudView == nil) return;
        g_hudView.baseSpeed = speed;
        [g_hudView setNeedsDisplay:YES];
    });
}
