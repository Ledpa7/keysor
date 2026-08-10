#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

// Rust ↔ C FFI 콜백 함수
extern void keysor_macos_on_lang_toggle(void);
extern void keysor_macos_on_speed_change(double delta);

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

// =========================================================================
// Keysor macOS HUD 설명 및 설정 팝업 윈도우 View
// =========================================================================
@interface KeysorHudView : NSView
@property (nonatomic, assign) BOOL langIsEnglish;
@property (nonatomic, assign) double baseSpeed;
@property (nonatomic, assign) NSPoint dragStartPoint;
@end

@implementation KeysorHudView

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

    if (key.length > 0) {
        NSFont *keyFont = [NSFont boldSystemFontOfSize:12.0];
        NSDictionary *keyAttrs = @{ NSFontAttributeName: keyFont, NSForegroundColorAttributeName: textColor };
        NSSize keySize = [key sizeWithAttributes:keyAttrs];
        NSRect keyRect = NSMakeRect(rect.origin.x + (rect.size.width - keySize.width)/2.0, rect.origin.y + rect.size.height - keySize.height - 4.0, keySize.width, keySize.height);
        [key drawInRect:keyRect withAttributes:keyAttrs];
    }

    if (desc.length > 0) {
        NSFont *descFont = [NSFont systemFontOfSize:9.0 weight:NSFontWeightMedium];
        NSColor *descColor = (category > 0) ? [NSColor whiteColor] : [NSColor colorWithCalibratedRed:0.33 green:0.33 blue:0.33 alpha:1.0];
        NSDictionary *descAttrs = @{ NSFontAttributeName: descFont, NSForegroundColorAttributeName: descColor };
        NSSize descSize = [desc sizeWithAttributes:descAttrs];
        NSRect descRect = NSMakeRect(rect.origin.x + (rect.size.width - descSize.width)/2.0, rect.origin.y + 4.0, descSize.width, descSize.height);
        [desc drawInRect:descRect withAttributes:descAttrs];
    }
}

static void drawHudButton(NSRect rect, NSString *label, NSColor *bgColor, NSColor *borderColor, NSColor *textColor) {
    NSBezierPath *path = [NSBezierPath bezierPathWithRoundedRect:rect xRadius:5.0 yRadius:5.0];
    [bgColor setFill];
    [path fill];
    [borderColor setStroke];
    [path setLineWidth:1.0];
    [path stroke];

    NSFont *font = [NSFont boldSystemFontOfSize:11.0];
    NSDictionary *attrs = @{ NSFontAttributeName: font, NSForegroundColorAttributeName: textColor };
    NSSize sz = [label sizeWithAttributes:attrs];
    NSRect r = NSMakeRect(rect.origin.x + (rect.size.width - sz.width)/2.0, rect.origin.y + (rect.size.height - sz.height)/2.0, sz.width, sz.height);
    [label drawInRect:r withAttributes:attrs];
}

- (void)drawRect:(NSRect)dirtyRect {
    [super drawRect:dirtyRect];

    // HUD 메인 배경 패널 (Dark Glassmorphic UI)
    NSRect bounds = [self bounds];
    NSBezierPath *bgPath = [NSBezierPath bezierPathWithRoundedRect:bounds xRadius:16.0 yRadius:16.0];
    [[NSColor colorWithCalibratedRed:0.07 green:0.07 blue:0.06 alpha:0.95] setFill];
    [bgPath fill];
    [[NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:0.9] setStroke]; // Neon Lime Green
    [bgPath setLineWidth:2.0];
    [bgPath stroke];

    // Y축 좌표계: AppKit 기본 (하단 0) -> 설계 좌표 (상단 0) 변환 (452 - y)
    CGFloat h = bounds.size.height;

    // 1. 타이틀 헤더
    NSString *titleStr = self.langIsEnglish ? @"Keysor, Keyboard & Cursor as One!" : @"Keysor, 키보드와 커서를 하나로!";
    NSFont *titleFont = [NSFont boldSystemFontOfSize:18.0];
    NSDictionary *titleAttrs = @{ NSFontAttributeName: titleFont, NSForegroundColorAttributeName: [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0] };
    [titleStr drawAtPoint:NSMakePoint(30, h - 30 - 24) withAttributes:titleAttrs];

    // Top Right 컨트롤 버튼들
    drawHudButton(NSMakeRect(430, h - 30 - 24, 110, 24), self.langIsEnglish ? @"Buy Pro" : @"프로 결제하기", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.24 green:0.25 blue:0.25 alpha:1.0], [NSColor colorWithCalibratedRed:0.53 green:0.53 blue:0.53 alpha:1.0]);
    drawHudButton(NSMakeRect(546, h - 30 - 24, 110, 24), self.langIsEnglish ? @"License" : @"라이선스 등록", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.24 green:0.25 blue:0.25 alpha:1.0], [NSColor colorWithCalibratedRed:0.53 green:0.53 blue:0.53 alpha:1.0]);
    drawHudButton(NSMakeRect(662, h - 30 - 24, 50, 24), self.langIsEnglish ? @"KO" : @"EN", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0]);
    drawHudButton(NSMakeRect(742, h - 10 - 20, 24, 20), @"-", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.24 green:0.25 blue:0.25 alpha:1.0], [NSColor whiteColor]);
    drawHudButton(NSMakeRect(772, h - 10 - 20, 24, 20), @"X", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.24 green:0.25 blue:0.25 alpha:1.0], [NSColor colorWithCalibratedRed:1.0 green:0.27 blue:0.0 alpha:1.0]);

    // 2. 5열 온스크린 키보드 조작 가이드 패널
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

    // 3. SPEED SENS (마우스 민감도 조절 패널)
    NSRect sensPanelRect = NSMakeRect(638, h - 80 - 264, 140, 264);
    NSBezierPath *sensPath = [NSBezierPath bezierPathWithRoundedRect:sensPanelRect xRadius:10.0 yRadius:10.0];
    [[NSColor colorWithCalibratedRed:0.09 green:0.09 blue:0.09 alpha:1.0] setFill];
    [sensPath fill];
    [[NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:0.8] setStroke];
    [sensPath setLineWidth:1.5];
    [sensPath stroke];

    // SPEED SENS 타이틀
    NSDictionary *sensTitleAttrs = @{ NSFontAttributeName: [NSFont boldSystemFontOfSize:13.0], NSForegroundColorAttributeName: [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0] };
    [@"SPEED SENS" drawAtPoint:NSMakePoint(660, h - 90 - 16) withAttributes:sensTitleAttrs];

    // 수치 디스플레이
    NSString *spdStr = [NSString stringWithFormat:@"%.1f", self.baseSpeed > 0 ? self.baseSpeed : 1.5];
    NSDictionary *spdAttrs = @{ NSFontAttributeName: [NSFont boldSystemFontOfSize:22.0], NSForegroundColorAttributeName: [NSColor whiteColor] };
    NSSize spdSz = [spdStr sizeWithAttributes:spdAttrs];
    [spdStr drawAtPoint:NSMakePoint(638 + (140 - spdSz.width)/2.0, h - 130 - 24) withAttributes:spdAttrs];

    // + / - 버튼
    drawHudButton(NSMakeRect(654, h - 175 - 28, 48, 28), @"+", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0]);
    drawHudButton(NSMakeRect(714, h - 175 - 28, 48, 28), @"-", [NSColor colorWithCalibratedRed:0.13 green:0.14 blue:0.14 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0], [NSColor colorWithCalibratedRed:0.68 green:1.0 blue:0.18 alpha:1.0]);

    // 4. 하단 안내 문구
    NSString *info1 = self.langIsEnglish ? @"※ All alphabet typing is blocked during mouse mode." : @"※ 마우스 모드 중 모든 알파벳 키 타이핑은 차단됩니다.";
    NSString *info2 = self.langIsEnglish ? @"• Press Caps Lock again to return to normal keyboard mode." : @"• Caps Lock을 한 번 더 누르면 일반 키보드 상태로 복귀합니다.";
    NSString *info3 = self.langIsEnglish ? @"• Automatically minimizes when mouse mode is active." : @"• 마우스 활성화 시 자동으로 최소화 됩니다.";

    NSDictionary *infoAttrs = @{ NSFontAttributeName: [NSFont systemFontOfSize:11.0], NSForegroundColorAttributeName: [NSColor colorWithCalibratedRed:0.6 green:0.6 blue:0.6 alpha:1.0] };
    [info1 drawAtPoint:NSMakePoint(30, h - 365 - 14) withAttributes:infoAttrs];
    [info2 drawAtPoint:NSMakePoint(30, h - 388 - 14) withAttributes:infoAttrs];
    [info3 drawAtPoint:NSMakePoint(30, h - 411 - 14) withAttributes:infoAttrs];
}

// 창 마우스 드래그 및 버튼 클릭 이벤트 처리
- (void)mouseDown:(NSEvent *)event {
    NSPoint p = [self convertPoint:[event locationInWindow] fromView:nil];
    CGFloat h = [self bounds].size.height;

    // 언어 변경 버튼 클릭
    if (NSPointInRect(p, NSMakeRect(662, h - 30 - 24, 50, 24))) {
        self.langIsEnglish = !self.langIsEnglish;
        [self setNeedsDisplay:YES];
        keysor_macos_on_lang_toggle();
        return;
    }

    // 최소화 버튼 클릭 (borderless 창은 miniaturize 불가 → orderOut으로 창 숨김)
    if (NSPointInRect(p, NSMakeRect(742, h - 10 - 20, 24, 20))) {
        [[self window] orderOut:nil];
        return;
    }

    // 닫기 버튼 클릭 (HUD X 버튼 클릭 시 프로세스 완전 종료)
    if (NSPointInRect(p, NSMakeRect(772, h - 10 - 20, 24, 20))) {
        NSLog(@"[Keysor ObjC] HUD Close X button clicked, exiting Keysor...");
        exit(0);
        return;
    }

    // Pro 구매 버튼 클릭
    if (NSPointInRect(p, NSMakeRect(430, h - 30 - 24, 110, 24))) {
        [[NSWorkspace sharedWorkspace] openURL:[NSURL URLWithString:@"https://keysor.vercel.app/#pricing"]];
        return;
    }

    // + 속도 증가
    if (NSPointInRect(p, NSMakeRect(654, h - 175 - 28, 48, 28))) {
        self.baseSpeed += 0.5;
        if (self.baseSpeed > 10.0) self.baseSpeed = 10.0;
        [self setNeedsDisplay:YES];
        keysor_macos_on_speed_change(0.5);
        return;
    }

    // - 속도 감수
    if (NSPointInRect(p, NSMakeRect(714, h - 175 - 28, 48, 28))) {
        self.baseSpeed -= 0.5;
        if (self.baseSpeed < 0.1) self.baseSpeed = 0.1;
        [self setNeedsDisplay:YES];
        keysor_macos_on_speed_change(-0.5);
        return;
    }

    // 윈도우 배경 드래그 이동 시작 포인트 기록
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
            NSRect mainScreenFrame = [[NSScreen mainScreen] frame];
            CGFloat hudX = (mainScreenFrame.size.width - 808.0) / 2.0;
            CGFloat hudY = (mainScreenFrame.size.height - 452.0) / 2.0;
            [g_hudWindow setFrameOrigin:NSMakePoint(hudX, hudY)];
            [g_hudWindow setAlphaValue:1.0];
            [g_hudWindow setLevel:NSModalPanelWindowLevel];
            [g_hudWindow makeKeyAndOrderFront:nil];
            [g_hudWindow orderFrontRegardless];
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
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        g_appDelegate = [[KeysorAppDelegate alloc] init];
        [NSApp setDelegate:g_appDelegate];
        [NSApp finishLaunching];

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

        // 2. HUD 키보드 가이드 & 설정 팝업 윈도우 생성 (808x452)
        NSRect mainScreenFrame = [[NSScreen mainScreen] frame];
        CGFloat hudX = (mainScreenFrame.size.width - 808.0) / 2.0;
        CGFloat hudY = (mainScreenFrame.size.height - 452.0) / 2.0;
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
        g_hudView.baseSpeed = 1.5;
        g_hudView.langIsEnglish = NO;
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
        NSRect displayBounds = [[NSScreen mainScreen] frame];
        double screen_h = displayBounds.size.height;
        // 커서 핫스팟 (16, 16) 정밀 동기화
        NSPoint origin = NSMakePoint(x - 16.0, screen_h - y - 20.0);
        [g_overlayWindow setFrameOrigin:origin];
    };

    if ([NSThread isMainThread]) {
        posBlock();
    } else {
        dispatch_async(dispatch_get_main_queue(), posBlock);
    }
}

void keysor_macos_ui_set_click_motion(int clickType) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_cursorView == nil) return;
        g_cursorView.cursorState = clickType;
        [g_cursorView setNeedsDisplay:YES];
    });
}

void keysor_macos_ui_update_speed_display(double speed) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (g_hudView == nil) return;
        g_hudView.baseSpeed = speed;
        [g_hudView setNeedsDisplay:YES];
    });
}
