//
//  hilen_text.m
//  Hilen
//
//  The system side of an engine text field. While a field is edited the user
//  types into a real UITextField, or a UITextView for a multiline field, that
//  sits exactly over the engine field. So the keyboard, autocorrect, text
//  composing and the selection handles all come from iOS.
//
//  The engine draws nothing of the text for that time. This side takes the
//  font, the size, the color and the place of the text from the engine, so the
//  switch between the 2 is not visible. Every change of the text goes to the
//  engine at once through `changed`.
//
//  The build script of the engine compiles this file into the engine library,
//  so this side and `system_field.rs` are always the same version.
//

#import <CoreText/CoreText.h>
#import <UIKit/UIKit.h>

// Must match `TextEdit` in `src/ui/mobile/ios.rs`. Lengths are in the
// pixels of the engine view.
typedef struct {
    float x;
    float y;
    float width;
    float height;

    const char* text;
    float text_size;
    float line_height;
    float red;
    float green;
    float blue;
    float alpha;

    // The gap between the left or right edge and the text, and above the
    // first line of a multiline field.
    float inset;
    float top_inset;

    // 0 left, 1 center, 2 right.
    int alignment;
    // 0 text, 1 numbers.
    int keyboard;

    bool secure;
    bool multiline;
    bool dark;

    // The bytes of the font file, alive for the whole process, and one name
    // for every different font. No bytes means the system font.
    const unsigned char* font;
    unsigned long font_length;
    const char* font_key;
    const unsigned int* variation_tags;
    const float* variation_values;
    int variation_count;
} HilenTextEdit;

typedef void (*HilenTextChanged)(const char* text);
typedef void (*HilenTextEvent)(void);

// A UITextField draws its text from the very edge, an engine field leaves a
// gap there.
@interface HilenTextField : UITextField
@property(nonatomic) CGFloat inset;
@end

@implementation HilenTextField
- (CGRect)textRectForBounds:(CGRect)bounds {
    return CGRectInset(bounds, self.inset, 0);
}
- (CGRect)editingRectForBounds:(CGRect)bounds {
    return CGRectInset(bounds, self.inset, 0);
}
@end

@interface HilenTextBridge : NSObject <UITextFieldDelegate, UITextViewDelegate>
@end

static HilenTextField* hilen_field = nil;
static UITextView* hilen_area = nil;
static HilenTextBridge* hilen_bridge = nil;
// The one of the 2 that is edited now, nil between editing sessions.
static UIView* hilen_edited = nil;
// The engine asked for the end itself, so `ended` must not tell it again.
static BOOL hilen_ending = NO;

static HilenTextChanged hilen_changed = NULL;
static HilenTextEvent hilen_returned = NULL;
static HilenTextEvent hilen_ended = NULL;

@implementation HilenTextBridge

- (void)fieldChanged:(UITextField*)field {
    if (hilen_changed) hilen_changed(field.text.UTF8String);
}

- (void)textViewDidChange:(UITextView*)area {
    if (hilen_changed) hilen_changed(area.text.UTF8String);
}

- (BOOL)textFieldShouldReturn:(UITextField*)field {
    if (hilen_returned) hilen_returned();
    return NO;
}

// The system can end the editing with no word from the engine, the hide key
// of an iPad keyboard does.
- (void)textFieldDidEndEditing:(UITextField*)field {
    if (!hilen_ending && hilen_ended) hilen_ended();
}

- (void)textViewDidEndEditing:(UITextView*)area {
    if (!hilen_ending && hilen_ended) hilen_ended();
}

@end

static UIView* hilen_text_host(void) {
    return UIApplication.sharedApplication.keyWindow.rootViewController.view;
}

static CGFloat hilen_text_scale(void) {
    return hilen_text_host().contentScaleFactor;
}

static void hilen_text_setup(void) {
    if (hilen_bridge) return;

    hilen_bridge = [HilenTextBridge new];

    hilen_field = [HilenTextField new];
    hilen_field.borderStyle = UITextBorderStyleNone;
    hilen_field.backgroundColor = UIColor.clearColor;
    hilen_field.delegate = hilen_bridge;
    hilen_field.hidden = YES;
    [hilen_field addTarget:hilen_bridge
                    action:@selector(fieldChanged:)
          forControlEvents:UIControlEventEditingChanged];

    hilen_area = [UITextView new];
    hilen_area.backgroundColor = UIColor.clearColor;
    hilen_area.textContainer.lineFragmentPadding = 0;
    hilen_area.delegate = hilen_bridge;
    hilen_area.hidden = YES;

    [hilen_text_host() addSubview:hilen_field];
    [hilen_text_host() addSubview:hilen_area];
}

// The engine font at `size`. Made from the same file the engine draws with, a
// font installed on the phone under the same name could be another version.
static UIFont* hilen_text_font(const HilenTextEdit* edit, CGFloat size) {
    if (edit->font == NULL || edit->font_length == 0) {
        return [UIFont systemFontOfSize:size];
    }

    static NSMutableDictionary<NSString*, id>* descriptors = nil;
    if (!descriptors) descriptors = [NSMutableDictionary new];

    NSString* key = [NSString stringWithUTF8String:edit->font_key];
    id descriptor = descriptors[key];

    if (!descriptor) {
        NSData* data = [NSData dataWithBytesNoCopy:(void*)edit->font
                                            length:edit->font_length
                                      freeWhenDone:NO];
        CTFontDescriptorRef plain = CTFontManagerCreateFontDescriptorFromData((__bridge CFDataRef)data);
        if (plain == NULL) {
            NSLog(@"hilen: the system cannot read font %@, the system font is used", key);
            return [UIFont systemFontOfSize:size];
        }

        if (edit->variation_count > 0) {
            NSMutableDictionary* variations = [NSMutableDictionary new];
            for (int i = 0; i < edit->variation_count; i++) {
                variations[@(edit->variation_tags[i])] = @(edit->variation_values[i]);
            }
            NSDictionary* attributes = @{(__bridge NSString*)kCTFontVariationAttribute : variations};
            CTFontDescriptorRef varied =
                CTFontDescriptorCreateCopyWithAttributes(plain, (__bridge CFDictionaryRef)attributes);
            CFRelease(plain);
            plain = varied;
        }

        descriptor = (__bridge_transfer id)plain;
        descriptors[key] = descriptor;
    }

    CTFontRef font = CTFontCreateWithFontDescriptor((__bridge CTFontDescriptorRef)descriptor, size, NULL);
    return (__bridge_transfer UIFont*)font;
}

static UIKeyboardType hilen_text_keyboard(int keyboard) {
    switch (keyboard) {
        // The plain number pad has no minus and no point.
        case 1: return UIKeyboardTypeNumbersAndPunctuation;
        default: return UIKeyboardTypeDefault;
    }
}

static NSTextAlignment hilen_text_alignment(int alignment) {
    switch (alignment) {
        case 0: return NSTextAlignmentLeft;
        case 2: return NSTextAlignmentRight;
        default: return NSTextAlignmentCenter;
    }
}

void hilen_ios_text_move(float x, float y, float width, float height) {
    CGFloat scale = hilen_text_scale();
    hilen_edited.frame = CGRectMake(x / scale, y / scale, width / scale, height / scale);
}

void hilen_ios_text_end(void) {
    if (!hilen_edited) return;

    // Resigning accepts a pending autocorrection, which reports one last
    // change. The callbacks stay set until that is through.
    hilen_ending = YES;
    [hilen_edited resignFirstResponder];
    // The engine view takes the keys the moment its subview lets them go,
    // it can be a first responder itself, and the keyboard would stay up.
    UIView* host = hilen_text_host();
    if (host.isFirstResponder) [host resignFirstResponder];
    hilen_edited.hidden = YES;
    hilen_ending = NO;

    hilen_edited = nil;
    hilen_changed = NULL;
    hilen_returned = NULL;
    hilen_ended = NULL;
    // A password must not stay in a view nobody sees.
    hilen_field.text = @"";
    hilen_area.text = @"";
}

void hilen_ios_text_begin(const HilenTextEdit* edit,
                          HilenTextChanged changed,
                          HilenTextEvent returned,
                          HilenTextEvent ended) {
    hilen_text_setup();
    hilen_ios_text_end();

    CGFloat scale = hilen_text_scale();
    UIFont* font = hilen_text_font(edit, edit->text_size / scale);
    UIColor* color = [UIColor colorWithRed:edit->red green:edit->green blue:edit->blue alpha:edit->alpha];
    NSString* text = [NSString stringWithUTF8String:edit->text];
    NSTextAlignment alignment = hilen_text_alignment(edit->alignment);
    UIKeyboardType keyboard = hilen_text_keyboard(edit->keyboard);
    UIKeyboardAppearance appearance = edit->dark ? UIKeyboardAppearanceDark : UIKeyboardAppearanceLight;

    if (edit->multiline) {
        NSMutableParagraphStyle* lines = [NSMutableParagraphStyle new];
        lines.alignment = alignment;
        lines.minimumLineHeight = edit->line_height / scale;
        lines.maximumLineHeight = edit->line_height / scale;
        NSDictionary* attributes = @{
            NSFontAttributeName : font,
            NSForegroundColorAttributeName : color,
            NSParagraphStyleAttributeName : lines,
        };

        hilen_area.textContainerInset =
            UIEdgeInsetsMake(edit->top_inset / scale, edit->inset / scale, 0, edit->inset / scale);
        hilen_area.attributedText = [[NSAttributedString alloc] initWithString:text attributes:attributes];
        hilen_area.typingAttributes = attributes;
        hilen_area.tintColor = color;
        hilen_area.keyboardType = keyboard;
        hilen_area.keyboardAppearance = appearance;
        hilen_edited = hilen_area;
    } else {
        hilen_field.inset = edit->inset / scale;
        hilen_field.font = font;
        hilen_field.textColor = color;
        hilen_field.tintColor = color;
        hilen_field.textAlignment = alignment;
        hilen_field.secureTextEntry = edit->secure;
        hilen_field.keyboardType = keyboard;
        hilen_field.keyboardAppearance = appearance;
        hilen_field.text = text;
        hilen_edited = hilen_field;
    }

    hilen_changed = changed;
    hilen_returned = returned;
    hilen_ended = ended;

    hilen_ios_text_move(edit->x, edit->y, edit->width, edit->height);
    hilen_edited.hidden = NO;
    [hilen_edited becomeFirstResponder];
}

// The engine changed the text of the edited field itself, a constraint that
// dropped a typed character or a `set_text` call.
void hilen_ios_text_set(const char* text) {
    NSString* string = [NSString stringWithUTF8String:text];

    if (hilen_edited == hilen_field) {
        if (![hilen_field.text isEqualToString:string]) hilen_field.text = string;
    } else if (hilen_edited == hilen_area) {
        if (![hilen_area.text isEqualToString:string]) {
            hilen_area.attributedText =
                [[NSAttributedString alloc] initWithString:string attributes:hilen_area.typingAttributes];
        }
    }
}
